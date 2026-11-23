//! Secret redaction for terminal output and echoed commands.
//!
//! There is exactly one redaction implementation in the workspace:
//! [`devdeck_audit::redact`]. Audit calls it before every append and the
//! terminal calls it before output reaches the grid, logs or audit entries.
//! Keeping a single implementation avoids a terminal-only policy fork that
//! could silently diverge from the audited one.

pub use devdeck_audit::redact;

/// PTY output is a byte stream; decode lossily before redacting so a partial
/// UTF-8 sequence at a chunk boundary can never leak a secret unredacted.
pub fn redact_bytes(bytes: &[u8]) -> String {
    redact(&String::from_utf8_lossy(bytes))
}

/// Redact a single already-decoded line (for command echoes and audit details).
pub fn redact_line(line: &str) -> String {
    redact(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planted_tokens_are_removed_from_output() {
        let line = "export API_TOKEN=ghp_fakeTestToken0123456789 host=localhost";
        let redacted = redact_line(line);
        assert!(!redacted.contains("ghp_fakeTestToken0123456789"));
        assert!(redacted.contains("host=localhost"));
    }

    #[test]
    fn env_and_key_shapes_are_redacted() {
        for line in [
            "SECRET_KEY=sk-fakeSecretValue1234",
            "DATABASE_URL=postgres://user:hunter2@db.local/app",
            "AWS_ACCESS_KEY_ID=AKIAFAKE1234567890",
            "password: hunter2",
            "Authorization: Bearer abcdef123456",
        ] {
            let redacted = redact_line(line);
            assert!(!redacted.contains("fakeSecretValue1234"), "{redacted}");
            assert!(!redacted.contains("hunter2"), "{redacted}");
            assert!(!redacted.contains("AKIAFAKE1234567890"), "{redacted}");
            assert!(!redacted.contains("abcdef123456"), "{redacted}");
        }
    }

    #[test]
    fn redacts_lossy_byte_chunks() {
        let bytes = b"token=abcd1234\xff\n";
        let redacted = redact_bytes(bytes);
        assert!(!redacted.contains("abcd1234"));
    }
}
