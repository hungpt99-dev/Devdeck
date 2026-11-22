//! Append-only audit log with a hash chain. Every tool call (built-in,
//! extension or agent) produces an entry. Secrets are redacted before write.
#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum Actor {
    Human,
    System,
    Agent { id: String },
    Extension { id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Allowed,
    Denied,
    AwaitingApproval,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: u64,
    pub timestamp_ms: u128,
    pub actor: Actor,
    pub action: String,
    pub tool: Option<String>,
    pub permission: Option<String>,
    pub risk: Option<String>,
    pub outcome: Outcome,
    pub detail: String,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("cannot read audit log: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot encode audit log: {0}")]
    Json(#[from] serde_json::Error),
    #[error("audit log hash chain is broken at sequence {0}")]
    BrokenChain(u64),
}

pub struct AuditLog {
    path: PathBuf,
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn open(dir: &Path) -> Result<Self, AuditError> {
        fs::create_dir_all(dir)?;
        let path = dir.join("audit.jsonl");
        let entries = if path.exists() {
            fs::read_to_string(&path)?
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(serde_json::from_str::<AuditEntry>)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        Ok(Self { path, entries })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Append an entry and atomically replace the log. Full rewrite keeps the
    /// hash chain simple; logs are compact and rotated by `devdeck doctor`.
    pub fn append(&mut self, mut entry: NewAuditEntry) -> Result<&AuditEntry, AuditError> {
        let seq = self.entries.len() as u64 + 1;
        entry.detail = redact(&entry.detail);
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());
        let mut record = AuditEntry {
            seq,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            actor: entry.actor,
            action: entry.action,
            tool: entry.tool,
            permission: entry.permission,
            risk: entry.risk,
            outcome: entry.outcome,
            detail: entry.detail,
            prev_hash,
            hash: String::new(),
        };
        let mut hasher = Sha256::new();
        hasher.update(record.prev_hash.as_bytes());
        hasher.update(record.timestamp_ms.to_string().as_bytes());
        hasher.update(record.actor_json().as_bytes());
        hasher.update(record.action.as_bytes());
        hasher.update(record.detail.as_bytes());
        record.hash = format!("{:x}", hasher.finalize());

        let mut contents = String::new();
        for existing in &self.entries {
            contents.push_str(&serde_json::to_string(existing)?);
            contents.push('\n');
        }
        contents.push_str(&serde_json::to_string(&record)?);
        contents.push('\n');

        let temp = self.path.with_extension("jsonl.tmp");
        fs::write(&temp, contents)?;
        fs::rename(&temp, &self.path)?;
        self.entries.push(record);
        Ok(self.entries.last().expect("entry just pushed"))
    }

    pub fn verify_chain(&self) -> Result<(), AuditError> {
        let mut previous = "genesis".to_string();
        for entry in &self.entries {
            let mut hasher = Sha256::new();
            hasher.update(previous.as_bytes());
            hasher.update(entry.timestamp_ms.to_string().as_bytes());
            hasher.update(entry.actor_json().as_bytes());
            hasher.update(entry.action.as_bytes());
            hasher.update(entry.detail.as_bytes());
            let expected = format!("{:x}", hasher.finalize());
            if entry.hash != expected || entry.prev_hash != previous {
                return Err(AuditError::BrokenChain(entry.seq));
            }
            previous = entry.hash.clone();
        }
        Ok(())
    }
}

impl AuditEntry {
    fn actor_json(&self) -> String {
        serde_json::to_string(&self.actor).unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct NewAuditEntry {
    pub actor: Actor,
    pub action: String,
    pub tool: Option<String>,
    pub permission: Option<String>,
    pub risk: Option<String>,
    pub outcome: Outcome,
    pub detail: String,
}

impl NewAuditEntry {
    pub fn new(actor: Actor, action: impl Into<String>) -> Self {
        Self {
            actor,
            action: action.into(),
            tool: None,
            permission: None,
            risk: None,
            outcome: Outcome::Allowed,
            detail: String::new(),
        }
    }
}

/// Replace credential-shaped values before anything is persisted.
/// Deliberately conservative: false positives are preferable to leaks.
pub fn redact(input: &str) -> String {
    // Markers and token shapes are ASCII; compare byte slices
    // case-insensitively so no Unicode case-mapping can shift offsets and
    // accidentally expose a suffix.
    const MARKERS: &[&str] = &[
        "bearer ",
        "authorization:",
        "auth-token",
        "access-token",
        "access_key",
        "access-key",
        "api_key",
        "api-key",
        "apikey",
        "client_secret",
        "client-secret",
        "connection_string",
        "database_url",
        "db_url",
        "passwd",
        "password",
        "private_key",
        "private key",
        "refresh_token",
        "secret",
        "session_token",
        "token",
    ];
    // `Authorization: Bearer <token>` has a second qualifier after the marker.
    const AUTH_SCHEMES: &[&str] = &["bearer ", "basic ", "token "];
    // Some tokens arrive without a surrounding `KEY=value` marker (pasted
    // credentials, tool output). Mask those shapes too.
    const TOKEN_SHAPES: &[&str] = &[
        "ghp_",
        "github_pat_",
        "gho_",
        "ghu_",
        "ghs_",
        "ghr_",
        "sk-",
        "sk_",
        "AKIA",
        "ASIA",
        "xoxb-",
        "xoxp-",
        "AIza",
    ];

    fn mask_value(out: &mut String, input: &str, index: &mut usize) {
        let bytes = input.as_bytes();
        let mut masked = false;
        while *index < bytes.len()
            && !bytes[*index].is_ascii_whitespace()
            && !matches!(bytes[*index], b'"' | b'\'' | b',' | b'}' | b';' | b')')
        {
            let ch = input[*index..].chars().next().expect("char boundary");
            out.push('*');
            masked = true;
            *index += ch.len_utf8();
        }
        if !masked {
            out.push_str("[redacted]");
        }
    }

    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        let marker = MARKERS.iter().find(|m| {
            bytes[i..].len() >= m.len() && bytes[i..i + m.len()].eq_ignore_ascii_case(m.as_bytes())
        });
        if let Some(marker) = marker {
            out.push_str(&input[i..i + marker.len()]);
            i += marker.len();
            while i < bytes.len() && matches!(bytes[i], b':' | b'=' | b' ' | b'\t' | b'"' | b'\'') {
                out.push(bytes[i] as char);
                i += 1;
            }
            if let Some(scheme) = AUTH_SCHEMES.iter().find(|s| {
                bytes[i..].len() >= s.len()
                    && bytes[i..i + s.len()].eq_ignore_ascii_case(s.as_bytes())
            }) {
                out.push_str(&input[i..i + scheme.len()]);
                i += scheme.len();
            }
            mask_value(&mut out, input, &mut i);
            continue;
        }
        let shape = TOKEN_SHAPES.iter().find(|s| {
            bytes[i..].len() >= s.len() && bytes[i..i + s.len()].eq_ignore_ascii_case(s.as_bytes())
        });
        if let Some(shape) = shape {
            out.push_str(&input[i..i + shape.len()]);
            i += shape.len();
            mask_value(&mut out, input, &mut i);
            continue;
        }
        let ch = input[i..].chars().next().expect("char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets_but_keeps_context() {
        let secret = "Bearer abcdefghijklmnopqrstuvwxyz";
        let redacted = redact(secret);
        assert!(!redacted.contains("abcdefghijklmnopqrstuvwxyz"));
        assert!(redacted.starts_with("Bearer "));
        let redacted2 = redact("password=hunter2 host=localhost");
        assert!(!redacted2.contains("hunter2"));
        assert!(redacted2.contains("host=localhost"));
    }

    #[test]
    fn hash_chain_detects_tampering() {
        let dir = std::env::temp_dir().join(format!("devdeck-audit-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut log = AuditLog::open(&dir).unwrap();
        log.append(NewAuditEntry::new(Actor::System, "startup"))
            .unwrap();
        log.append(NewAuditEntry::new(
            Actor::Agent { id: "a1".into() },
            "tool.call",
        ))
        .unwrap();
        assert_eq!(log.entries().len(), 2);
        assert!(log.verify_chain().is_ok());
        let mut tampered = log.entries().to_vec();
        tampered[0].detail = "changed".into();
        let fake = AuditLog {
            path: log.path().to_owned(),
            entries: tampered,
        };
        assert!(matches!(
            fake.verify_chain(),
            Err(AuditError::BrokenChain(1))
        ));
        let reloaded = AuditLog::open(&dir).unwrap();
        assert!(reloaded.verify_chain().is_ok());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn planted_token_appears_in_zero_audit_entries() {
        let token = "ghp_fakeAuditToken0123456789";
        let dir = std::env::temp_dir().join(format!("devdeck-audit-secret-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut log = AuditLog::open(&dir).unwrap();
        let mut entry = NewAuditEntry::new(
            Actor::Agent {
                id: "terminal".into(),
            },
            "tool.call",
        );
        entry.tool = Some("terminal.spawn".into());
        entry.detail = format!("command=echo API_TOKEN={token} | policy-dangerous");
        log.append(entry).unwrap();
        assert!(log.entries().iter().all(|e| !e.detail.contains(token)));
        let on_disk = fs::read_to_string(log.path()).unwrap();
        assert!(!on_disk.contains(token));
        let reloaded = AuditLog::open(&dir).unwrap();
        assert!(reloaded.entries().iter().all(|e| !e.detail.contains(token)));
        fs::remove_dir_all(&dir).ok();
    }
}
