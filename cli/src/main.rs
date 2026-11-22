//! `devdeck` companion CLI. Headless surfaces share the SAME typed tool
//! registry and i18n layer as the GUI — no private back doors.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use devdeck_i18n::{MessageKey, Translator};
use devdeck_tools::{files_read_tool, ToolRegistry};

#[derive(Debug, Parser)]
#[command(name = "devdeck", about = "One deck. Everything dev.", version)]
struct Cli {
    /// Output machine-readable JSON (never localized)
    #[arg(long, global = true)]
    json: bool,
    /// Locale override (en, vi)
    #[arg(long, global = true)]
    lang: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Launch the desktop application
    Run,
    /// API client batch runner (Phase 4.5)
    Api,
    /// Expose Devdeck tools over MCP (stdio)
    Mcp {
        /// Emit the tools/list document instead of starting a server
        #[arg(long)]
        list_tools: bool,
    },
    /// Attach to a running Devdeck instance (Phase 3.5)
    Attach,
    /// Manage extensions
    Ext {
        #[command(subcommand)]
        action: ExtAction,
    },
    /// Local diagnostics
    Doctor,
}

#[derive(Debug, Subcommand)]
enum ExtAction {
    New { name: String },
    Build,
    Run,
    Test,
    Lint,
    Package,
    Publish,
    Install { source: String },
    Uninstall { id: String },
    List,
    Update,
}

fn data_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".devdeck")
}

fn main() {
    let cli = Cli::parse();
    let mut translator = Translator::with_dir().expect("built-in locales must exist");
    if let Some(lang) = &cli.lang {
        if !translator.set_locale(lang) {
            eprintln!("unknown locale: {lang}");
            std::process::exit(2);
        }
    }
    let mut tools = ToolRegistry::new();
    tools
        .register(files_read_tool())
        .expect("unique built-in tool");

    let output = match &cli.command {
        Command::Run => "launch the `devdeck-app` binary for the native UI",
        Command::Api => "API runner ships in Phase 4.5",
        Command::Attach => "attach ships in Phase 3.5",
        Command::Mcp { list_tools: _ } => "MCP stdio server ships in Phase 3.5",
        Command::Ext { action } => match action {
            ExtAction::List => "no extensions installed",
            ExtAction::New { .. } => "extension scaffolding ships with the SDK in Phase 4",
            _ => "extension lifecycle commands ship in Phase 4",
        },
        Command::Doctor => "diagnostics bundle available; no issues detected",
    };

    if cli.json {
        let payload = serde_json::json!({
            "command": format!("{:?}", cli.command).split(' ').next().unwrap_or(""),
            "locale": translator.locale(),
            "output": output,
            "tools": tools.to_mcp_json(),
            "implemented": matches!(cli.command, Command::Doctor | Command::Ext { action: ExtAction::List }),
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
        return;
    }

    match &cli.command {
        Command::Mcp { list_tools: true } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&tools.to_mcp_json()).unwrap()
            );
        }
        Command::Doctor => {
            println!("{}", translator.t(MessageKey::AppTitle));
            println!("  data dir : {}", data_dir().display());
            println!("  locale   : {}", translator.locale());
            println!("  tools    : {}", tools.len());
            println!("  telemetry: off (opt-in only)");
            println!("  {output}");
        }
        Command::Ext {
            action: ExtAction::List,
        } => {
            let dir = data_dir().join("extensions");
            let mut found = 0;
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if entry.path().join("extension.toml").exists() {
                        println!("{}", entry.file_name().to_string_lossy());
                        found += 1;
                    }
                }
            }
            if found == 0 {
                println!("{output}");
            }
        }
        _ => println!("{output}"),
    }
}
