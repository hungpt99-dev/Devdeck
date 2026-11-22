//! Bounded, deduplicated recent-project list backed by the config store.
//!
//! A project is identified by its canonical root path: two spellings of the
//! same directory (`/x/./y` and `/x/y`) always collapse into one entry. The
//! list is sorted newest-first and never exceeds [`MAX_RECENT_PROJECTS`].
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{detect_workspace, AppConfig, ConfigError, Event, EventBus, WorkspaceInfo};

/// Hard cap enforced on load, on open and on save.
pub const MAX_RECENT_PROJECTS: usize = 10;

/// One recently opened project. `last_opened` is Unix time in milliseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProject {
    pub root: PathBuf,
    pub last_opened: u64,
}

/// Resolve `path` to a stable identifier. Existing paths are canonicalized;
/// missing paths fall back to a lexical absolute path so a soon-to-be-created
/// project can still be recorded without a filesystem race.
pub fn canonical_root(path: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(path) {
        return canonical;
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other.as_os_str()),
        }
    }
    resolved
}

/// Deduplicate by canonical root, keep the newest timestamp for each root,
/// sort newest-first and cap the result. Safe to call on untrusted config.
pub fn normalize(projects: &[RecentProject]) -> Vec<RecentProject> {
    let mut newest: BTreeMap<PathBuf, u64> = BTreeMap::new();
    for project in projects {
        let root = canonical_root(&project.root);
        newest
            .entry(root)
            .and_modify(|time| *time = (*time).max(project.last_opened))
            .or_insert(project.last_opened);
    }
    let mut result: Vec<RecentProject> = newest
        .into_iter()
        .map(|(root, last_opened)| RecentProject { root, last_opened })
        .collect();
    result.sort_by(|a, b| {
        b.last_opened
            .cmp(&a.last_opened)
            .then_with(|| a.root.cmp(&b.root))
    });
    result.truncate(MAX_RECENT_PROJECTS);
    result
}

/// Record an open in place. Returns `true` when the list actually changed.
pub fn record_open(projects: &mut Vec<RecentProject>, root: &Path, now_ms: u64) -> bool {
    let mut next = projects.clone();
    next.push(RecentProject {
        root: canonical_root(root),
        last_opened: now_ms,
    });
    let next = normalize(&next);
    let changed = next != *projects;
    *projects = next;
    changed
}

/// Open a project through the single config writer: update recents, persist
/// atomically, detect workspace markers, then emit `Event::WorkspaceOpened`.
/// The event is only published after a successful save, so a failed write can
/// never claim a project was opened.
pub fn open_project(
    config_path: &Path,
    root: &Path,
    events: &EventBus,
    now_ms: u64,
) -> Result<WorkspaceInfo, ConfigError> {
    let mut config = AppConfig::load(config_path)?;
    record_open(&mut config.recent_projects, root, now_ms);
    config.save(config_path)?;
    let info = detect_workspace(&canonical_root(root));
    events.publish(Event::WorkspaceOpened {
        root: info.root.clone(),
    });
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[test]
    fn recents_are_capped_at_ten_and_newest_first() {
        let mut projects = Vec::new();
        for i in 0..25 {
            record_open(
                &mut projects,
                Path::new(&format!("/projects/p{i}")),
                1000 + i,
            );
        }
        assert_eq!(projects.len(), MAX_RECENT_PROJECTS);
        assert_eq!(projects[0].root, PathBuf::from("/projects/p24"));
        assert_eq!(
            projects.last().unwrap().root,
            PathBuf::from("/projects/p15")
        );
        assert!(projects
            .windows(2)
            .all(|w| w[0].last_opened >= w[1].last_opened));
    }

    #[test]
    fn recents_deduplicate_by_canonical_root() {
        let dir = std::env::temp_dir().join(format!("devdeck-recents-{}", std::process::id()));
        fs::create_dir_all(dir.join("nested")).unwrap();
        let mut projects = Vec::new();
        assert!(record_open(
            &mut projects,
            &dir.join("nested").join(".."),
            100
        ));
        assert!(record_open(&mut projects, &dir, 200));
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].root, fs::canonicalize(&dir).unwrap());
        assert_eq!(projects[0].last_opened, 200);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn opening_a_project_persists_recent_and_emits_event() {
        let dir = std::env::temp_dir().join(format!("devdeck-open-{}", std::process::id()));
        let project = dir.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("Cargo.toml"), "[package]").unwrap();
        let config_path = dir.join("config.json");
        let bus = EventBus::new(8);
        let mut receiver = bus.subscribe();
        let info = open_project(&config_path, &project, &bus, now_ms()).unwrap();
        assert_eq!(info.languages, vec!["rust"]);
        assert!(matches!(
            receiver.try_recv(),
            Ok(Event::WorkspaceOpened { root }) if root == fs::canonicalize(&project).unwrap()
        ));
        assert_eq!(
            AppConfig::load(&config_path).unwrap().recent_projects.len(),
            1
        );
        fs::remove_dir_all(&dir).ok();
    }
}
