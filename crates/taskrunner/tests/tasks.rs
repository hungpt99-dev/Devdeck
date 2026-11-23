//! Integration tests for the terminal-command task runner: parsing, cwd,
//! exit status, approval decisions and audit entries through the real F3 PTY.

use std::fs;
use std::path::{Path, PathBuf};

use devdeck_audit::{Actor, AuditLog, Outcome};
use devdeck_core::{
    detect_workspace, PermissionDecision, PermissionPolicy, PolicyOverride, WorkspaceConfig,
    WorkspaceInfo,
};
use devdeck_i18n::Translator;
use devdeck_taskrunner::{list, load, run, TaskError, TaskRunOutcome};
use devdeck_terminal::TerminalSettings;

fn translator(locale: &str) -> Translator {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../locales");
    let mut translator = Translator::new(&dir).unwrap();
    assert!(translator.set_locale(locale));
    translator
}

fn fixture(name: &str) -> (PathBuf, AuditLog) {
    let base = std::env::temp_dir().join(format!(
        "devdeck-taskrunner-{}-{}",
        std::process::id(),
        name
    ));
    let _ = fs::remove_dir_all(&base);
    let audit_dir = base.join("audit");
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(&audit_dir).unwrap();
    (base, AuditLog::open(&audit_dir).unwrap())
}

fn config(root: &Path, body: &str) {
    fs::write(WorkspaceConfig::path_in(root), body).unwrap();
}

fn workspace(root: &Path) -> WorkspaceInfo {
    detect_workspace(root)
}

fn cwd_command(marker: &Path) -> String {
    #[cfg(windows)]
    {
        format!("cd > \"{}\"", marker.display())
    }
    #[cfg(not(windows))]
    {
        format!("pwd > {}", marker.display())
    }
}

#[tokio::test]
async fn commands_run_in_their_own_workspace_root_and_status_propagates() {
    let (base, mut log) = fixture("cwd-status");
    let first = base.join("first");
    let other = base.join("other");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&other).unwrap();
    let marker = first.join("cwd.txt");
    let other_marker = other.join("cwd.txt");
    config(
        &first,
        &format!(
            "[tasks]\nwhere = \"{}\"\nfail = \"exit 7\"\n",
            cwd_command(&marker)
        ),
    );
    config(
        &other,
        &format!("[tasks]\nwhere = \"{}\"\n", cwd_command(&other_marker)),
    );
    // Same task name in another project must never be used as a fallback.
    assert_eq!(
        list(&first, &translator("en")).unwrap(),
        vec!["fail", "where"]
    );

    let outcome = run(
        "where",
        &workspace(&first),
        &PermissionPolicy::default(),
        &mut log,
        Actor::Human,
        TerminalSettings::default(),
        &translator("en"),
    )
    .await
    .unwrap();
    match &outcome {
        TaskRunOutcome::Completed { task, status, .. } => {
            assert_eq!(task, "where");
            assert_eq!(*status, Some(0));
        }
        other => panic!("task did not complete: {other:?}"),
    }
    assert_eq!(outcome.status(), Some(0));
    let recorded_cwd = fs::read_to_string(&marker).unwrap();
    assert!(
        recorded_cwd.contains(&first.to_string_lossy().to_string()),
        "cwd was not the workspace root: {recorded_cwd}"
    );
    assert!(!other_marker.exists(), "executed the neighbouring project");

    let failure = run(
        "fail",
        &workspace(&first),
        &PermissionPolicy::default(),
        &mut log,
        Actor::Human,
        TerminalSettings::default(),
        &translator("en"),
    )
    .await
    .unwrap();
    assert_eq!(failure.status(), Some(7));
    assert!(log.verify_chain().is_ok());
    let _ = fs::remove_dir_all(&base);
}

#[tokio::test]
async fn dangerous_command_is_asked_and_never_executed() {
    let (base, mut log) = fixture("approval");
    let root = base.join("workspace");
    fs::create_dir_all(&root).unwrap();
    let marker = root.join("should-not-exist.txt");
    let command = if cfg!(windows) {
        format!("rm -rf ./nope & echo ran > {}", marker.display())
    } else {
        format!("rm -rf ./nope && touch {}", marker.display())
    };
    config(&root, &format!("[tasks]\ncleanup = \"{command}\"\n"));

    let outcome = run(
        "cleanup",
        &workspace(&root),
        &PermissionPolicy::default(),
        &mut log,
        Actor::Human,
        TerminalSettings::default(),
        &translator("en"),
    )
    .await
    .unwrap();
    match outcome {
        TaskRunOutcome::RequiresApproval {
            task,
            command: shown,
            decision,
        } => {
            assert_eq!(task, "cleanup");
            assert_eq!(shown, command, "the exact command must be shown");
            assert!(matches!(decision, PermissionDecision::Ask { .. }));
        }
        other => panic!("dangerous task was auto-executed: {other:?}"),
    }
    assert!(!marker.exists(), "a dangerous task started a process");
    let entry = log.entries().last().unwrap();
    assert_eq!(entry.tool.as_deref(), Some("tasks.run"));
    assert_eq!(entry.outcome, Outcome::AwaitingApproval);
    assert_eq!(entry.risk.as_deref(), Some("Medium"));
    assert!(entry.detail.contains("task=cleanup"));
    assert!(log.verify_chain().is_ok());
    let _ = fs::remove_dir_all(&base);
}

#[tokio::test]
async fn deny_override_never_starts_a_process_and_is_audited() {
    let (base, mut log) = fixture("deny");
    let root = base.join("workspace");
    fs::create_dir_all(&root).unwrap();
    let marker = root.join("should-not-exist.txt");
    config(
        &root,
        &format!("[tasks]\nsafe = \"touch {}\"\n", marker.display()),
    );
    let mut policy = PermissionPolicy::default();
    policy.set_override("tasks.run".into(), PolicyOverride::Deny);

    let outcome = run(
        "safe",
        &workspace(&root),
        &policy,
        &mut log,
        Actor::Human,
        TerminalSettings::default(),
        &translator("en"),
    )
    .await
    .unwrap();
    assert!(matches!(
        outcome,
        TaskRunOutcome::RequiresApproval {
            decision: PermissionDecision::Deny { .. },
            ..
        }
    ));
    assert!(!marker.exists());
    assert_eq!(log.entries().last().unwrap().outcome, Outcome::Denied);
    let _ = fs::remove_dir_all(&base);
}

#[tokio::test]
async fn allowed_run_audits_the_exact_command_and_exit_status() {
    let (base, mut log) = fixture("audit");
    let root = base.join("workspace");
    fs::create_dir_all(&root).unwrap();
    config(&root, "[tasks]\nok = \"echo done\"\n");

    let outcome = run(
        "ok",
        &workspace(&root),
        &PermissionPolicy::default(),
        &mut log,
        Actor::Human,
        TerminalSettings::default(),
        &translator("en"),
    )
    .await
    .unwrap();
    match &outcome {
        TaskRunOutcome::Completed {
            command, status, ..
        } => {
            assert_eq!(command, "echo done");
            assert_eq!(*status, Some(0));
        }
        other => panic!("task was not allowed: {other:?}"),
    }
    let entry = log.entries().last().unwrap();
    assert_eq!(entry.tool.as_deref(), Some("tasks.run"));
    assert_eq!(entry.outcome, Outcome::Allowed);
    assert_eq!(entry.permission.as_deref(), Some("Allow"));
    assert!(entry.detail.contains("task=ok"));
    assert!(entry.detail.contains("command=echo done"));
    let on_disk = fs::read_to_string(log.path()).unwrap();
    assert!(on_disk.contains("tasks.run"));
    assert!(log.verify_chain().is_ok());
    let _ = fs::remove_dir_all(&base);
}

#[tokio::test]
async fn missing_config_unknown_task_and_malformed_command_are_typed_and_localized() {
    let (base, mut log) = fixture("errors");
    let root = base.join("workspace");
    fs::create_dir_all(&root).unwrap();
    let en = translator("en");
    let vi = translator("vi");

    let missing = load(&root, &en).unwrap_err();
    assert!(matches!(missing, TaskError::ConfigMissing { .. }));
    assert!(missing.to_string().contains("config is missing"));
    let missing_vi = load(&root, &vi).unwrap_err().to_string();
    assert!(missing_vi.contains("Thiếu cấu hình"), "{missing_vi}");
    assert!(!missing_vi.contains("[task-"));

    let no_root = load(&base.join("absent"), &en).unwrap_err();
    assert!(matches!(no_root, TaskError::WorkspaceRootMissing { .. }));

    config(&root, "[tasks]\nok = \"echo hi\"\n");
    let unknown = load(&root, &en).unwrap();
    assert_eq!(unknown.len(), 1);
    let unknown_err = run(
        "missing-task",
        &workspace(&root),
        &PermissionPolicy::default(),
        &mut log,
        Actor::Human,
        TerminalSettings::default(),
        &en,
    )
    .await
    .unwrap_err();
    assert!(matches!(unknown_err, TaskError::UnknownTask { .. }));
    assert!(unknown_err.to_string().contains("Unknown task"));

    config(&root, "[tasks]\nbroken = \"   \"\n");
    let malformed = load(&root, &en).unwrap_err();
    assert!(matches!(malformed, TaskError::MalformedCommand { .. }));
    let malformed_vi = load(&root, &vi).unwrap_err().to_string();
    assert!(malformed_vi.contains("lệnh để trống"), "{malformed_vi}");

    config(&root, "[tasks]\nok = 7\n");
    assert!(matches!(
        load(&root, &en),
        Err(TaskError::MalformedConfig { .. })
    ));
    config(&root, "schema_version = 99\n[tasks]\nok = \"echo hi\"\n");
    assert!(matches!(
        load(&root, &en),
        Err(TaskError::UnsupportedVersion { version: 99, .. })
    ));
    let _ = fs::remove_dir_all(&base);
}
