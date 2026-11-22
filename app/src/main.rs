//! Devdeck native application shell — Phase 1 vertical slice.
//! All user-facing strings come from `devdeck_i18n`; all capabilities from
//! `devdeck_tools` with a permission decision and an audit entry.
#![forbid(unsafe_code)]

use std::path::PathBuf;

use devdeck_audit::{Actor, AuditLog, NewAuditEntry, Outcome};
use devdeck_core::{
    audit_tool_call, detect_workspace, AppConfig, EventBus, PermissionDecision, PermissionPolicy,
    ThemeName, WorkspaceInfo,
};
use devdeck_i18n::{MessageKey, Translator};
use devdeck_tools::{files_read_tool, ToolRegistry};
use devdeck_ui::{command_palette, sidebar, status_bar, CommandItem, Tokens};
use iced::keyboard::{Event as KeyEvent, Key};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Settings, Size, Subscription, Task};

struct AppState {
    config: AppConfig,
    translator: Translator,
    tools: ToolRegistry,
    policy: PermissionPolicy,
    audit: AuditLog,
    workspace: WorkspaceInfo,
    events: EventBus,
    selected: usize,
    palette_open: bool,
    query: String,
    highlight: usize,
    last_action: String,
    // Cached localized view data: owned by state so `view` can borrow it for
    // the whole lifetime of the returned Element.
    nav_entries: Vec<(String, String)>,
    commands: Vec<CommandItem>,
    filtered_commands: Vec<CommandItem>,
    palette_placeholder: String,
    palette_empty: String,
}

#[derive(Debug, Clone)]
enum Message {
    Nav(usize),
    TogglePalette,
    Query(String),
    Execute(String),
    ExecuteHighlighted,
    SetLocale(String),
    CycleTheme,
    Key(KeyEvent),
}

fn refresh_localized(app: &mut AppState) {
    app.nav_entries = nav_labels(app)
        .into_iter()
        .map(|label| (label.clone(), label))
        .collect();
    app.commands = commands(app);
    app.filtered_commands = devdeck_ui::filter_commands(&app.query, &app.commands)
        .into_iter()
        .cloned()
        .collect();
    app.palette_placeholder = app.translator.t(MessageKey::CommandPalettePlaceholder);
    app.palette_empty = app.translator.t(MessageKey::CommandPaletteEmpty);
}

fn refresh_filtered(app: &mut AppState) {
    app.filtered_commands = devdeck_ui::filter_commands(&app.query, &app.commands)
        .into_iter()
        .cloned()
        .collect();
}

fn app_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".devdeck")
}

fn boot() -> (AppState, Task<Message>) {
    let dir = app_dir();
    let config = AppConfig::load(&dir.join("config.json")).unwrap_or_default();
    let translator = Translator::with_dir().expect("built-in locales en/vi must exist");
    let mut tools = ToolRegistry::new();
    tools
        .register(files_read_tool())
        .expect("built-in tool name is unique");
    let policy = PermissionPolicy::from_config(&config);
    let mut audit = AuditLog::open(&dir).expect("audit log must be writable");
    let workspace =
        detect_workspace(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let events = EventBus::new(64);
    let mut entry = NewAuditEntry::new(Actor::System, "app.started");
    entry.detail = format!("locale={} tools={}", config.locale, tools.len());
    let _ = audit.append(entry);
    let mut state = AppState {
        config,
        translator,
        tools,
        policy,
        audit,
        workspace,
        events,
        selected: 0,
        palette_open: false,
        query: String::new(),
        highlight: 0,
        last_action: String::new(),
        nav_entries: Vec::new(),
        commands: Vec::new(),
        filtered_commands: Vec::new(),
        palette_placeholder: String::new(),
        palette_empty: String::new(),
    };
    refresh_localized(&mut state);
    (state, Task::none())
}

fn nav_labels(app: &AppState) -> Vec<String> {
    [
        MessageKey::NavTerminal,
        MessageKey::NavEditor,
        MessageKey::NavGit,
        MessageKey::NavApi,
        MessageKey::NavDatabase,
        MessageKey::NavDocker,
        MessageKey::NavExtensions,
        MessageKey::NavSettings,
    ]
    .into_iter()
    .map(|key| app.translator.t(key))
    .collect()
}

fn commands(app: &AppState) -> Vec<CommandItem> {
    let mut items: Vec<CommandItem> = nav_labels(app)
        .into_iter()
        .enumerate()
        .map(|(i, label)| CommandItem {
            id: format!("nav:{i}"),
            hint: "navigation".into(),
            keywords: vec![
                "terminal".into(),
                "editor".into(),
                "git".into(),
                "api".into(),
                "database".into(),
                "docker".into(),
                "extensions".into(),
                "settings".into(),
            ],
            label,
        })
        .collect();
    for tool in app.tools.list() {
        items.push(CommandItem {
            id: format!("tool:{}", tool.name),
            label: app.translator.t_args(&tool.command_palette_key, &[]),
            hint: tool.name.clone(),
            keywords: vec![tool.description.clone(), tool.name.clone()],
        });
    }
    items.push(CommandItem {
        id: "theme:cycle".into(),
        label: app.translator.t(MessageKey::SwitchTheme),
        hint: "theme".into(),
        keywords: vec!["dark".into(), "light".into(), "contrast".into()],
    });
    items.push(CommandItem {
        id: "lang:toggle".into(),
        label: app.translator.t(MessageKey::SwitchLanguage),
        hint: "en / vi".into(),
        keywords: vec!["language".into(), "locale".into(), "tiếng việt".into()],
    });
    items
}

fn execute(app: &mut AppState, id: &str) {
    if let Some(index) = id.strip_prefix("nav:") {
        if let Ok(index) = index.parse::<usize>() {
            app.selected = index;
        }
        app.palette_open = false;
    } else if let Some(name) = id.strip_prefix("tool:") {
        if let Some(tool) = app.tools.get(name).cloned() {
            let decision = app.policy.decide(&tool, "");
            let _ = audit_tool_call(
                &mut app.audit,
                Actor::Human,
                &tool,
                &decision,
                "invoked from command palette",
            );
            app.last_action = match decision {
                PermissionDecision::Allow => format!("{} → allowed", tool.name),
                PermissionDecision::Ask { reason } | PermissionDecision::Deny { reason } => {
                    format!("{} → {reason}", tool.name)
                }
            };
            app.events
                .publish(devdeck_core::Event::ToolCalled { name: name.into() });
        }
        app.palette_open = false;
    } else if id == "theme:cycle" {
        app.config.theme = match app.config.theme {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::HighContrast,
            ThemeName::HighContrast => ThemeName::Dark,
        };
        let _ = app.config.save(&app_dir().join("config.json"));
        app.palette_open = false;
    } else if id == "lang:toggle" {
        let next = if app.translator.locale() == "vi" {
            "en"
        } else {
            "vi"
        };
        set_locale(app, next);
    }
}

fn set_locale(app: &mut AppState, locale: &str) {
    if app.translator.set_locale(locale) {
        app.config.locale = app.translator.locale().to_string();
        let _ = app.config.save(&app_dir().join("config.json"));
        app.events.publish(devdeck_core::Event::LocaleChanged {
            locale: app.config.locale.clone(),
        });
        let mut entry = NewAuditEntry::new(Actor::Human, "locale.changed");
        entry.outcome = Outcome::Allowed;
        entry.detail = app.config.locale.clone();
        let _ = app.audit.append(entry);
    }
    app.palette_open = false;
    refresh_localized(app);
}

fn is_shortcut(key: &KeyEvent) -> bool {
    matches!(key, KeyEvent::KeyPressed { key: Key::Character(c), modifiers, .. }
        if modifiers.command() && c.eq_ignore_ascii_case("k"))
}

fn update(app: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::Nav(index) => {
            app.selected = index;
            app.palette_open = false;
        }
        Message::TogglePalette => {
            app.palette_open = !app.palette_open;
            app.query.clear();
            app.highlight = 0;
        }
        Message::Query(query) => {
            app.query = query;
            app.highlight = 0;
            refresh_filtered(app);
        }
        Message::Execute(id) => execute(app, &id),
        Message::ExecuteHighlighted => {
            let all = commands(app);
            let results = devdeck_ui::filter_commands(&app.query, &all);
            if let Some(item) = results.get(app.highlight) {
                let id = item.id.clone();
                execute(app, &id);
            }
        }
        Message::SetLocale(locale) => set_locale(app, &locale),
        Message::CycleTheme => execute(app, "theme:cycle"),
        Message::Key(event) => match event {
            KeyEvent::KeyPressed {
                key: Key::Named(named),
                ..
            } => {
                use iced::keyboard::key::Named;
                if !app.palette_open {
                    return Task::none();
                }
                match named {
                    Named::Escape => app.palette_open = false,
                    Named::ArrowDown => {
                        let count = devdeck_ui::filter_commands(&app.query, &app.commands).len();
                        if count > 0 {
                            app.highlight = (app.highlight + 1) % count;
                        }
                    }
                    Named::ArrowUp => {
                        let count = devdeck_ui::filter_commands(&app.query, &app.commands).len();
                        if count > 0 {
                            app.highlight = (app.highlight + count - 1) % count;
                        }
                    }
                    Named::Enter => {
                        let all = commands(app);
                        let results = devdeck_ui::filter_commands(&app.query, &all);
                        if let Some(item) = results.get(app.highlight) {
                            let id = item.id.clone();
                            execute(app, &id);
                        }
                    }
                    _ => {}
                }
            }
            event if is_shortcut(&event) => {
                return iced::Task::done(Message::TogglePalette);
            }
            _ => {}
        },
    }
    Task::none()
}

fn view(app: &AppState) -> Element<'_, Message> {
    let tokens = Tokens::for_theme(app.config.theme);
    let labels = &app.nav_entries;

    let header = row![
        text(app.translator.t(MessageKey::AppTitle)).size(22),
        text(app.translator.t(MessageKey::AppTagline)).size(14),
        iced::widget::Space::new().width(Length::Fill),
        button(text(app.translator.t(MessageKey::SwitchLanguage)).size(14))
            .on_press(Message::SetLocale(if app.translator.locale() == "vi" {
                "en".into()
            } else {
                "vi".into()
            }))
            .style(move |_, _| iced::widget::button::Style {
                background: Some(iced::Background::Color(tokens.surface_alt)),
                text_color: tokens.text,
                ..Default::default()
            }),
        button(text(app.translator.t(MessageKey::SwitchTheme)).size(14))
            .on_press(Message::CycleTheme)
            .style(move |_, _| iced::widget::button::Style {
                background: Some(iced::Background::Color(tokens.surface_alt)),
                text_color: tokens.text,
                ..Default::default()
            }),
    ]
    .spacing(devdeck_ui::SPACE_MD)
    .padding(devdeck_ui::SPACE_MD);

    let content: Element<'_, Message> = if app.palette_open {
        command_palette(
            tokens,
            &app.query,
            &app.filtered_commands,
            devdeck_ui::PaletteLabels {
                placeholder: &app.palette_placeholder,
                empty: &app.palette_empty,
            },
            Message::Query,
            Message::Execute,
            || Message::ExecuteHighlighted,
        )
    } else {
        let names: Vec<String> = app.tools.list().map(|tool| tool.name.clone()).collect();
        let tool_list = if names.is_empty() {
            text("—").size(14)
        } else {
            text(names.join("\n")).size(14)
        };
        container(
            column![
                text(
                    labels
                        .get(app.selected)
                        .map(|e| e.0.clone())
                        .unwrap_or_default()
                )
                .size(20),
                text(app.translator.t(MessageKey::Welcome)).size(16),
                iced::widget::Space::new().height(Length::Fixed(devdeck_ui::SPACE_MD)),
                text(format!(
                    "{} · {} · {}",
                    app.workspace.root.display(),
                    app.workspace.languages.join(", "),
                    app.translator.locale()
                ))
                .size(13),
                tool_list,
                text(&app.last_action).size(13),
            ]
            .spacing(devdeck_ui::SPACE_SM),
        )
        .padding(devdeck_ui::SPACE_LG)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };

    let count = app
        .translator
        .t_count(MessageKey::StatusTools, app.tools.len());
    let status = format!("{} · {}", app.translator.t(MessageKey::StatusReady), count);

    column![
        container(header).style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(tokens.background)),
            text_color: Some(tokens.text),
            ..Default::default()
        }),
        row![
            sidebar(tokens, &app.nav_entries, app.selected, Message::Nav),
            content
        ]
        .height(Length::Fill),
        status_bar(tokens, status),
    ]
    .into()
}

fn subscription(_app: &AppState) -> Subscription<Message> {
    iced::keyboard::listen().map(Message::Key)
}

fn theme(app: &AppState) -> iced::Theme {
    match app.config.theme {
        ThemeName::Dark | ThemeName::HighContrast => iced::Theme::Dark,
        ThemeName::Light => iced::Theme::Light,
    }
}

fn title(app: &AppState) -> String {
    app.translator.t(MessageKey::AppTitle)
}

fn main() -> iced::Result {
    let _ = tracing_subscriber::fmt::try_init();
    iced::application(boot, update, view)
        .title(title)
        .theme(theme)
        .subscription(subscription)
        .settings(Settings::default())
        .window(iced::window::Settings {
            size: Size::new(1200.0, 800.0),
            ..Default::default()
        })
        .run()
}
