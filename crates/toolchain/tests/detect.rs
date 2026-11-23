//! Fake-PATH integration tests: detection must never consult the real host.
//! Shell-script fixtures need a POSIX executable bit, so these run on Unix;
//! parsing/type tests in the crate itself are cross-platform.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use devdeck_toolchain::{DetectionFailure, DetectionStatus, Detector, ToolId};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct FakePath {
    root: PathBuf,
}

impl FakePath {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "devdeck-ind216-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fake PATH fixture");
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn write_script(&self, name: &str, body: &str) {
        let path = self.root.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake tool");
        let mut permissions = fs::metadata(&path)
            .expect("fake tool metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("make fake tool executable");
    }

    fn print_version(&self, name: &str, output: &str) {
        self.write_script(name, &format!("printf '%s\\n' '{output}'"));
    }

    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.root)
            .expect("read fixture")
            .map(|entry| {
                entry
                    .expect("fixture entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for FakePath {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn detector(path: &FakePath) -> Detector {
    Detector::new()
        .with_timeout(Duration::from_secs(2))
        .with_path(path.path())
}

fn failure_of(result: &devdeck_toolchain::DetectedTool) -> DetectionFailure {
    match result.status {
        DetectionStatus::NotDetected { reason } => reason,
        DetectionStatus::Detected { .. } => panic!("expected a not-detected result"),
    }
}

#[test]
fn detects_fixed_allow_list_from_fake_path() {
    let fake = FakePath::new();
    fake.print_version("cargo", "cargo 1.85.0 (d73d2caf9 2024-12-31)");
    fake.print_version("git", "git version 2.43.0");
    fake.print_version("go", "go version go1.22.5 linux/amd64");
    fake.print_version("mise", "mise 2025.1.2 linux-x64");
    fake.print_version("node", "v22.13.1");
    fake.print_version("python", "Python 3.12.3");
    fake.print_version("rustc", "rustc 1.85.0 (abc 2025-01-01)");

    let results = detector(&fake).detect_all();

    let ids: Vec<ToolId> = results.iter().map(|result| result.tool).collect();
    assert_eq!(ids, ToolId::ALL.to_vec());
    let versions: Vec<&str> = results
        .iter()
        .map(|result| result.version().expect("detected version"))
        .collect();
    assert_eq!(
        versions,
        ["1.85.0", "2.43.0", "1.22.5", "2025.1.2", "22.13.1", "3.12.3", "1.85.0"]
    );
    assert!(results.iter().all(|result| result.failure().is_none()));
}

#[test]
fn missing_tools_are_typed_not_detected() {
    let fake = FakePath::new(); // empty PATH
    let results = detector(&fake).detect_all();
    assert_eq!(results.len(), ToolId::ALL.len());
    for result in results {
        assert!(!result.is_detected());
        assert_eq!(failure_of(&result), DetectionFailure::NotFound);
        assert_eq!(result.version(), None);
    }
}

#[test]
fn unparseable_and_failing_probes_are_typed() {
    let fake = FakePath::new();
    fake.print_version("git", "no version here");
    fake.write_script("node", "printf 'v22.13.1\\n'; exit 1");

    let git = detector(&fake).detect(ToolId::Git);
    let node = detector(&fake).detect(ToolId::Node);
    assert_eq!(failure_of(&git), DetectionFailure::UnparseableVersion);
    assert_eq!(failure_of(&node), DetectionFailure::ExitFailure);
    // Raw output (including stderr) is never carried to consumers.
    let serialized = serde_json_like(&git);
    assert!(!serialized.contains("no version here"));
    assert!(!serialized.contains("stderr"));
}

fn serde_json_like(result: &devdeck_toolchain::DetectedTool) -> String {
    format!("{:?}", result)
}

#[test]
fn timeout_is_typed_and_bounded() {
    let fake = FakePath::new();
    // Drop `--version`; absolute path because the fake PATH is deliberately empty.
    fake.write_script("git", "shift $#\nexec /bin/sleep 5");

    let started = Instant::now();
    let result = Detector::new()
        .with_timeout(Duration::from_millis(100))
        .with_path(fake.path())
        .detect(ToolId::Git);

    assert_eq!(failure_of(&result), DetectionFailure::Timeout);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "probe must be killed"
    );
}

#[test]
fn read_only_home_and_path_still_detect() {
    let fake = FakePath::new();
    fake.print_version("git", "git version 2.43.0");
    let home = fake.path().join("home");
    fs::create_dir_all(&home).expect("read-only HOME fixture");
    let mut permissions = fs::metadata(&home).expect("home metadata").permissions();
    permissions.set_mode(0o500); // readable, not writable
    fs::set_permissions(&home, permissions).expect("make HOME read-only");

    let result = detector(&fake).with_home(&home).detect(ToolId::Git);

    assert_eq!(result.version(), Some("2.43.0"));
    assert_eq!(fs::read_dir(&home).expect("read HOME").count(), 0);
}

#[test]
fn detection_does_not_mutate_environment_or_fixture() {
    let fake = FakePath::new();
    fake.print_version("git", "git version 2.43.0");
    let before_path = std::env::var_os("PATH");
    let before_home = std::env::var_os("HOME");
    let entries_before = fake.entries();

    let _ = detector(&fake).detect_all();

    assert_eq!(std::env::var_os("PATH"), before_path);
    assert_eq!(std::env::var_os("HOME"), before_home);
    assert_eq!(fake.entries(), entries_before);
}
