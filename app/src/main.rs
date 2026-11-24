//! Devdeck native application shell — Phase 2 Foundation six-region layout.
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
use devdeck_ui::{
    activity_rail, bottom_panel, command_palette, context_bar, shell, status_bar, tab_bar,
    CommandItem, Region, ShellMessage, ShellParts, ShellState, Tokens, BODY_SIZE, SPACE_LG,
    SPACE_MD, SPACE_SM,
};
use iced::keyboard::{Event as KeyEvent, Key};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Length, Settings, Size, Subscription, Task};

fn window_size() -> Size {
    Size::new(1200.0, 800.0)
}

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
    shell: ShellState,
    window: Size,
    // Cached localized view data: owned by state so `view` can borrow it for
    // the whole lifetime of the returned Element.
    nav_entries: Vec<(String, String)>,
    commands: Vec<CommandItem>,
    filtered_commands: Vec<CommandItem>,
    palette_placeholder: String,
    palette_empty: String,
    panel_labels: Vec<String>,
    panel_placeholders: Vec<String>,
    context_title: String,
    context_lines: Vec<String>,
    workspace_tab: String,
    header_project: String,
    status_locale: String,
    status_permission: String,
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
    Shell(ShellMessage),
    Raw(iced::event::Event),
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
    app.panel_labels = [
        MessageKey::PanelTerminal,
        MessageKey::PanelProblems,
        MessageKey::PanelOutput,
        MessageKey::PanelLogs,
    ]
    .into_iter()
    .map(|key| app.translator.t(key))
    .collect();
    app.panel_placeholders = [
        MessageKey::PanelTerminalPlaceholder,
        MessageKey::PanelProblemsPlaceholder,
        MessageKey::PanelOutputPlaceholder,
        MessageKey::PanelLogsPlaceholder,
    ]
    .into_iter()
    .map(|key| app.translator.t(key))
    .collect();
    app.context_title = app.translator.t(MessageKey::ContextTitle);
    app.workspace_tab = app.translator.t(MessageKey::WorkspaceTab);
    app.header_project = app.translator.t(MessageKey::HeaderProject);
    app.status_locale = app.translator.t(MessageKey::StatusLocale);
    app.status_permission = app.translator.t(MessageKey::StatusPermission);
    app.context_lines = context_lines(app);
}

fn context_lines(app: &AppState) -> Vec<String> {
    let project = app
        .workspace
        .root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    vec![
        format!("{}: {}", app.header_project, project),
        format!(
            "{}: {}",
            app.translator.t(MessageKey::NavEditor),
            app.workspace.languages.join(", ")
        ),
        format!("{}: {}", app.status_locale, app.translator.locale()),
    ]
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
    let mut translator = Translator::with_dir().expect("built-in locales en/vi must exist");
    // The persisted locale is the source of truth at startup; Translator::new
    // otherwise picks up the host locale. Keep runtime switching and boot in sync.
    let _ = translator.set_locale(&config.locale);
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
    let window = window_size();
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
        shell: ShellState::default(),
        window,
        nav_entries: Vec::new(),
        commands: Vec::new(),
        filtered_commands: Vec::new(),
        palette_placeholder: String::new(),
        palette_empty: String::new(),
        panel_labels: Vec::new(),
        panel_placeholders: Vec::new(),
        context_title: String::new(),
        context_lines: Vec::new(),
        workspace_tab: String::new(),
        header_project: String::new(),
        status_locale: String::new(),
        status_permission: String::new(),
    };
    refresh_localized(&mut state);
    state.shell.clamp_to(window);
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
    for (id, key) in [
        ("shell:panel.toggle", MessageKey::CommandTogglePanel),
        ("shell:context.toggle", MessageKey::CommandToggleContext),
        ("shell:sidebar.toggle", MessageKey::CommandToggleSidebar),
        ("shell:panel.grow", MessageKey::CommandResizePanelUp),
        ("shell:panel.shrink", MessageKey::CommandResizePanelDown),
        ("shell:focus.next", MessageKey::CommandFocusNext),
        ("shell:tab.next", MessageKey::CommandNextTab),
    ] {
        items.push(CommandItem {
            id: id.into(),
            label: app.translator.t(key),
            hint: String::new(),
            keywords: vec!["shell".into(), "layout".into()],
        });
    }
    items
}

/// Palette command ids map to shell messages so layout commands have the same
/// keyboard-first parity as every other action.
fn shell_message_for(id: &str) -> Option<ShellMessage> {
    match id {
        "shell:panel.toggle" => Some(ShellMessage::TogglePanel),
        "shell:context.toggle" => Some(ShellMessage::ToggleContext),
        "shell:sidebar.toggle" => Some(ShellMessage::ToggleSidebar),
        "shell:panel.grow" => Some(ShellMessage::PanelResizeBy { delta: 40.0 }),
        "shell:panel.shrink" => Some(ShellMessage::PanelResizeBy { delta: -40.0 }),
        "shell:focus.next" => Some(ShellMessage::FocusNext),
        "shell:tab.next" => Some(ShellMessage::Focus(Region::Center)),
        _ => None,
    }
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
    } else if let Some(message) = shell_message_for(id) {
        app.shell.apply(message, app.window);
        app.palette_open = false;
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
    app.shell.clamp_to(app.window);
}

fn is_shortcut(key: &KeyEvent) -> bool {
    matches!(key, KeyEvent::KeyPressed { key: Key::Character(c), modifiers, .. }
        if modifiers.command() && c.eq_ignore_ascii_case("k"))
}

fn is_panel_toggle(key: &KeyEvent) -> bool {
    matches!(key, KeyEvent::KeyPressed { key: Key::Character(c), modifiers, .. }
        if modifiers.command() && !modifiers.shift() && c.eq_ignore_ascii_case("j"))
}

fn is_context_toggle(key: &KeyEvent) -> bool {
    matches!(key, KeyEvent::KeyPressed { key: Key::Character(c), modifiers, .. }
        if modifiers.command() && modifiers.shift() && c.eq_ignore_ascii_case("b"))
}

fn is_sidebar_toggle(key: &KeyEvent) -> bool {
    matches!(key, KeyEvent::KeyPressed { key: Key::Character(c), modifiers, .. }
        if modifiers.command() && modifiers.shift() && c.eq_ignore_ascii_case("s"))
}

fn is_next_tab(key: &KeyEvent) -> bool {
    matches!(key, KeyEvent::KeyPressed { key: Key::Named(named), modifiers, .. }
        if modifiers.command() && *named == iced::keyboard::key::Named::Tab)
}

fn focus_step(key: &KeyEvent) -> Option<isize> {
    match key {
        KeyEvent::KeyPressed {
            key: Key::Named(named),
            modifiers,
            ..
        } if *named == iced::keyboard::key::Named::F6 => {
            Some(if modifiers.shift() { -1 } else { 1 })
        }
        _ => None,
    }
}

fn resize_step(key: &KeyEvent) -> Option<f32> {
    match key {
        KeyEvent::KeyPressed {
            key: Key::Character(character),
            modifiers,
            ..
        } if character == "`" && modifiers.command() => {
            Some(if modifiers.shift() { -40.0 } else { 40.0 })
        }
        _ => None,
    }
}

fn update(app: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::Raw(event) => match event {
            iced::event::Event::Window(iced::window::Event::Resized(size)) => {
                app.window = size;
                app.shell.clamp_to(size);
            }
            iced::event::Event::Mouse(iced::mouse::Event::CursorMoved { position })
                if app.shell.dragging_splitter =>
            {
                app.shell.apply(
                    ShellMessage::PanelResizeFromCursor {
                        cursor_y: position.y,
                    },
                    app.window,
                );
            }
            iced::event::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )) if app.shell.dragging_splitter => {
                app.shell.apply(ShellMessage::SplitterReleased, app.window);
            }
            _ => {}
        },
        Message::Shell(message) => app.shell.apply(message, app.window),
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
        Message::Key(event) => {
            if is_shortcut(&event) {
                return Task::done(Message::TogglePalette);
            }
            if is_panel_toggle(&event) {
                return Task::done(Message::Shell(ShellMessage::TogglePanel));
            }
            if is_context_toggle(&event) {
                return Task::done(Message::Shell(ShellMessage::ToggleContext));
            }
            if is_sidebar_toggle(&event) {
                return Task::done(Message::Shell(ShellMessage::ToggleSidebar));
            }
            if is_next_tab(&event) {
                return Task::done(Message::Shell(ShellMessage::TabSelected(
                    app.shell.active_tab + 1,
                )));
            }
            if let Some(step) = focus_step(&event) {
                let message = if step > 0 {
                    ShellMessage::FocusNext
                } else {
                    ShellMessage::FocusPrevious
                };
                return Task::done(Message::Shell(message));
            }
            if let Some(delta) = resize_step(&event) {
                return Task::done(Message::Shell(ShellMessage::PanelResizeBy { delta }));
            }
            if app.palette_open {
                if let KeyEvent::KeyPressed {
                    key: Key::Named(named),
                    ..
                } = event
                {
                    use iced::keyboard::key::Named;
                    match named {
                        Named::Escape => app.palette_open = false,
                        Named::ArrowDown => {
                            let count =
                                devdeck_ui::filter_commands(&app.query, &app.commands).len();
                            if count > 0 {
                                app.highlight = (app.highlight + 1) % count;
                            }
                        }
                        Named::ArrowUp => {
                            let count =
                                devdeck_ui::filter_commands(&app.query, &app.commands).len();
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
            }
        }
    }
    Task::none()
}

fn view(app: &AppState) -> Element<'_, Message> {
    let tokens = Tokens::for_theme(app.config.theme);
    let labels = &app.nav_entries;

    let header: Element<'_, Message> = row![
        text(app.translator.t(MessageKey::AppTitle)).size(22),
        text(format!(
            "{}: {}",
            app.header_project,
            app.workspace
                .root
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
        ))
        .size(14),
        text(app.translator.t(MessageKey::AppTagline)).size(14),
        Space::new().width(Length::Fill),
        button(text(app.translator.t(MessageKey::PaletteShortcut)).size(14))
            .on_press(Message::TogglePalette)
            .style(move |_, _| iced::widget::button::Style {
                background: Some(iced::Background::Color(tokens.surface_alt)),
                text_color: tokens.text,
                ..Default::default()
            }),
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
    .spacing(SPACE_MD)
    .padding(SPACE_SM)
    .into();

    // Activity-rail launchers are abbreviated from localized nav labels, so
    // no literal label ever reaches the screen.
    let rail_entries: Vec<String> = app
        .nav_entries
        .iter()
        .map(|(label, _)| label.chars().take(2).collect::<String>())
        .collect();
    let activity = activity_rail(
        tokens,
        &rail_entries,
        app.shell.activity_selected,
        |index| Message::Shell(ShellMessage::ActivitySelected(index)),
    );

    let sidebar = devdeck_ui::sidebar(tokens, labels, app.selected, Message::Nav);

    let center: Element<'_, Message> = if app.palette_open {
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
        let tabs = vec![app.workspace_tab.clone()];
        let names: Vec<String> = app.tools.list().map(|tool| tool.name.clone()).collect();
        let tool_list = if names.is_empty() {
            text("—").size(14)
        } else {
            text(names.join("\n")).size(14)
        };
        let body = container(
            column![
                text(
                    labels
                        .get(app.selected)
                        .map(|entry| entry.0.clone())
                        .unwrap_or_default()
                )
                .size(20),
                text(app.translator.t(MessageKey::Welcome)).size(16),
                Space::new().height(Length::Fixed(SPACE_MD)),
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
            .spacing(SPACE_SM),
        )
        .padding(SPACE_LG)
        .width(Length::Fill)
        .height(Length::Fill);
        column![
            tab_bar(tokens, &tabs, app.shell.active_tab, |index| Message::Shell(
                ShellMessage::TabSelected(index)
            )),
            body,
        ]
        .into()
    };

    let panel_placeholder = app
        .panel_placeholders
        .get(app.shell.active_panel.index())
        .cloned()
        .unwrap_or_default();
    let panel_body: Element<'_, Message> = container(text(panel_placeholder).size(BODY_SIZE))
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    let panel = bottom_panel(
        tokens,
        app.shell.active_panel,
        &app.panel_labels,
        panel_body,
        |page| Message::Shell(ShellMessage::PanelSelected(page)),
    );

    let context = context_bar(tokens, app.context_title.clone(), &app.context_lines);

    let count = app
        .translator
        .t_count(MessageKey::StatusTools, app.tools.len());
    let status = format!(
        "{} · {} · {}: {} · {}",
        app.translator.t(MessageKey::StatusReady),
        count,
        app.status_locale,
        app.translator.locale(),
        app.status_permission,
    );
    let status = status_bar(tokens, status);

    shell::shell(
        tokens,
        &app.shell,
        app.window,
        ShellParts {
            header,
            activity,
            sidebar,
            center,
            context,
            panel,
            status,
        },
        Message::Shell,
    )
}

fn subscription(_app: &AppState) -> Subscription<Message> {
    Subscription::batch([
        iced::keyboard::listen().map(Message::Key),
        iced::event::listen().map(Message::Raw),
    ])
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
            size: window_size(),
            ..Default::default()
        })
        .run()
}
