//! Deterministic crash injection around the config temp/rename boundary.
//!
//! The parent test spawns this same test binary once per write checkpoint.
//! Each child kills itself inside the real `AppConfig::save_with_probe` path,
//! so the on-disk document is exactly what a process death at that point
//! would leave. Every survivor must be the complete old or complete new v2
//! document — never a truncated or half-migrated file.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use devdeck_core::{
    AppConfig, PolicyOverride, TaskDefaults, TerminalDefaults, ThemeName, WriteCheckpoint,
    CONFIG_SCHEMA_VERSION, CONFIG_WRITE_CHECKPOINTS,
};
use serde_json::Value;

const CRASH_ENV: &str = "DEVDECK_CONFIG_CRASH_POINT";
const DIR_ENV: &str = "DEVDECK_CONFIG_CRASH_DIR";

fn old_config() -> AppConfig {
    AppConfig {
        schema_version: CONFIG_SCHEMA_VERSION,
        locale: "en".into(),
        theme: ThemeName::Dark,
        telemetry_enabled: false,
        tool_policies: BTreeMap::new(),
        recent_projects: Vec::new(),
        terminal: TerminalDefaults::default(),
        task_defaults: TaskDefaults::default(),
    }
}

fn new_config() -> AppConfig {
    let mut policies = BTreeMap::new();
    policies.insert("files.read".to_string(), PolicyOverride::Ask);
    policies.insert("projects.open".to_string(), PolicyOverride::Deny);
    AppConfig {
        schema_version: CONFIG_SCHEMA_VERSION,
        locale: "vi".into(),
        theme: ThemeName::Light,
        telemetry_enabled: true,
        tool_policies: policies,
        recent_projects: vec![devdeck_core::RecentProject {
            root: PathBuf::from("/projects/alpha"),
            last_opened: 1_700_000_000_000,
        }],
        terminal: TerminalDefaults {
            font_size: 16,
            scrollback_lines: 20_000,
            shell: None,
        },
        task_defaults: TaskDefaults {
            auto_run_on_open: true,
            max_parallel: 8,
        },
    }
}

fn child_mode() -> bool {
    env::var_os(CRASH_ENV).is_some()
}

/// Runs only in a spawned child: write `new_config` and die at one checkpoint.
#[test]
fn crash_writer_child() {
    let Ok(point) = env::var(CRASH_ENV) else {
        return;
    };
    let point: usize = point.parse().expect("numeric crash point");
    assert!(point < CONFIG_WRITE_CHECKPOINTS);
    let dir = PathBuf::from(env::var(DIR_ENV).expect("crash directory"));
    new_config()
        .save_with_probe(&dir.join("config.json"), |checkpoint: WriteCheckpoint| {
            if checkpoint.index == point {
                // Abrupt termination: no unwinding, no buffered flush, no
                // rename cleanup. `ulimit -c` is zero in the test container.
                eprintln!("crash writer dying at checkpoint {checkpoint:?}");
                process::abort();
            }
        })
        .expect("every checkpoint fires before save returns");
    process::abort();
}

/// Kill a writer at all 100 deterministic points and verify the file at each.
#[test]
fn every_crash_point_leaves_a_complete_config() {
    if child_mode() {
        return;
    }
    let executable = env::current_exe().expect("test binary path");
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = env::temp_dir().join(format!("devdeck-crash-{}-{stamp}", process::id()));
    let old_value = serde_json::to_value(old_config()).unwrap();
    let new_value = serde_json::to_value(new_config()).unwrap();

    for point in 0..CONFIG_WRITE_CHECKPOINTS {
        let dir = root.join(format!("point-{point:03}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, serde_json::to_string_pretty(&old_value).unwrap()).unwrap();

        let status = Command::new(&executable)
            .arg("--quiet")
            .env(CRASH_ENV, point.to_string())
            .env(DIR_ENV, &dir)
            .status()
            .expect("spawn crash writer");
        assert!(!status.success(), "writer survived checkpoint {point}");

        let raw = fs::read_to_string(&path).expect("config still readable");
        let value: Value = serde_json::from_str(&raw).unwrap_or_else(|e| {
            panic!("truncated config at checkpoint {point}: {e}\nbytes: {raw}")
        });
        assert_eq!(value["schema_version"], CONFIG_SCHEMA_VERSION);
        for field in [
            "locale",
            "theme",
            "telemetry_enabled",
            "tool_policies",
            "recent_projects",
            "terminal",
            "task_defaults",
        ] {
            assert!(
                value.get(field).is_some(),
                "missing {field} at checkpoint {point}"
            );
        }
        assert!(
            value == old_value || value == new_value,
            "half-written config at checkpoint {point}: {value}"
        );
        assert_eq!(AppConfig::load(&path).unwrap().schema_version, 2);
    }

    fs::remove_dir_all(&root).ok();
}
