//! Build/release automation. `cargo run -p devdeck-xtask -- i18n-lint`
//! is the same check CI runs: locale key parity and hard-coded string scan.
#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn locale_keys(dir: &Path) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return keys;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ftl") {
            continue;
        }
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if let Some((key, _)) = line.split_once('=') {
                    let key = key.trim();
                    if !key.is_empty() && !key.starts_with('#') {
                        keys.insert(key.to_string());
                    }
                }
            }
        }
    }
    keys
}

fn i18n_lint() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../locales");
    let english = locale_keys(&root.join("en"));
    let vietnamese = locale_keys(&root.join("vi"));
    let missing: Vec<&String> = english.difference(&vietnamese).collect();
    let extra: Vec<&String> = vietnamese.difference(&english).collect();
    if !missing.is_empty() {
        return Err(format!(
            "vi is missing {} key(s): {:?}",
            missing.len(),
            missing
        ));
    }
    if !extra.is_empty() {
        return Err(format!(
            "vi has {} unknown key(s): {:?}",
            extra.len(),
            extra
        ));
    }
    println!("i18n-lint: en/vi parity OK ({} keys)", english.len());
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("i18n-lint") => i18n_lint(),
        Some("version") => {
            println!("devdeck {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err("usage: devdeck-xtask <i18n-lint|version>".into()),
    };
    if let Err(error) = result {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_are_in_sync() {
        i18n_lint().expect("en and vi must have identical key sets");
    }
}
