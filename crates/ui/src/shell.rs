//! Six-region native shell layout (IND-212).
//!
//! The shell owns region geometry, splitter/collapsed state and the small
//! primitives used by every region. It deliberately contains no business
//! logic and no hard-coded colors: every color comes from [`Tokens`], and
//! every user-facing string is supplied by the caller (the app resolves it
//! through `devdeck_i18n`). The pure geometry functions are the contract the
//! unit tests pin down.
#![forbid(unsafe_code)]

use iced::mouse::Interaction;
use iced::widget::{button, column, container, mouse_area, row, text, Column, MouseArea, Space};
use iced::{Background, Border, Element, Length, Size};

use crate::{
    panel_style, Tokens, BODY_SIZE, SIDEBAR_WIDTH, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XS,
    STATUS_HEIGHT,
};

/// Left activity rail. Narrow enough to be only a strip of launchers.
pub const ACTIVITY_RAIL_WIDTH: f32 = 48.0;
/// Optional right-hand context bar.
pub const CONTEXT_BAR_WIDTH: f32 = 240.0;
/// Default height of the collapsible bottom panel.
pub const BOTTOM_PANEL_DEFAULT_HEIGHT: f32 = 240.0;
/// A dragged panel never collapses below this; collapse is an explicit action.
pub const BOTTOM_PANEL_MIN_HEIGHT: f32 = 64.0;
/// The center workspace always keeps at least this much height.
pub const MIN_WORKSPACE_HEIGHT: f32 = 120.0;
/// Splitter grab strip between center workspace and bottom panel.
pub const SPLITTER_THICKNESS: f32 = SPACE_XS;
/// Height reserved for the header, shared with `app::view` so geometry math
/// and the actual rendered layout agree.
pub const HEADER_HEIGHT: f32 = SPACE_LG * 3.0;
/// Height of the workspace tab bar and of the bottom-panel tab bar.
pub const TAB_BAR_HEIGHT: f32 = SPACE_LG * 2.0;
/// Height of one activity-rail launcher.
pub const REGION_BUTTON_HEIGHT: f32 = SPACE_LG * 3.0;

/// Vertical space left for the center workspace plus bottom panel once the
/// header and status bar have taken their fixed share.
pub fn split_area_height(window_height: f32) -> f32 {
    (window_height - HEADER_HEIGHT - STATUS_HEIGHT).max(0.0)
}

/// Clamp a requested bottom-panel height to the available split area.
/// Returns `0.0` when the window is too short to host a usable panel.
pub fn clamp_panel_height(requested: f32, window_height: f32) -> f32 {
    let max_panel = split_area_height(window_height) - SPLITTER_THICKNESS - MIN_WORKSPACE_HEIGHT;
    if max_panel < BOTTOM_PANEL_MIN_HEIGHT {
        return 0.0;
    }
    requested.clamp(BOTTOM_PANEL_MIN_HEIGHT, max_panel)
}

/// Resolve the two heights in the vertical split: `(workspace, panel)`.
/// A hidden panel gives the whole split area to the workspace.
pub fn split_heights(window_height: f32, requested: f32, visible: bool) -> (f32, f32) {
    let available = split_area_height(window_height);
    if !visible {
        return (available, 0.0);
    }
    let panel = clamp_panel_height(requested, window_height);
    if panel == 0.0 {
        return (available, 0.0);
    }
    (available - SPLITTER_THICKNESS - panel, panel)
}

/// Panel height implied by dragging the splitter to an absolute window `y`.
pub fn panel_height_from_cursor(cursor_y: f32, window_height: f32) -> f32 {
    let split_bottom = HEADER_HEIGHT + split_area_height(window_height);
    clamp_panel_height(split_bottom - cursor_y, window_height)
}

/// Width of the center region after the fixed left and optional right rails.
pub fn center_width(window_width: f32, context_visible: bool, sidebar_visible: bool) -> f32 {
    let mut width = window_width - ACTIVITY_RAIL_WIDTH;
    if sidebar_visible {
        width -= SIDEBAR_WIDTH;
    }
    if context_visible {
        width -= CONTEXT_BAR_WIDTH;
    }
    width.max(0.0)
}

/// Bottom-panel pages. Placeholder content for later phases is supplied by
/// the app; this enum only owns selection state and tab ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Terminal,
    Problems,
    Output,
    Logs,
}

impl Panel {
    pub const ALL: [Panel; 4] = [Self::Terminal, Self::Problems, Self::Output, Self::Logs];

    pub fn index(self) -> usize {
        match self {
            Self::Terminal => 0,
            Self::Problems => 1,
            Self::Output => 2,
            Self::Logs => 3,
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::Terminal)
    }

    pub fn next(self) -> Self {
        Self::from_index((self.index() + 1) % Self::ALL.len())
    }
}

/// Keyboard-focusable shell regions, in Tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    ActivityRail,
    PrimarySidebar,
    Center,
    ContextBar,
    BottomPanel,
    StatusBar,
}

impl Region {
    pub const ALL: [Region; 6] = [
        Self::ActivityRail,
        Self::PrimarySidebar,
        Self::Center,
        Self::ContextBar,
        Self::BottomPanel,
        Self::StatusBar,
    ];

    pub fn is_visible(self, state: &ShellState) -> bool {
        match self {
            Self::ContextBar => state.context_visible,
            Self::BottomPanel => state.panel_visible,
            _ => true,
        }
    }
}

/// Messages the shell emits. The app wraps these in its own `Message` enum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellMessage {
    ActivitySelected(usize),
    TabSelected(usize),
    PanelSelected(Panel),
    TogglePanel,
    ToggleContext,
    ToggleSidebar,
    SplitterPressed,
    SplitterReleased,
    PanelResizeFromCursor { cursor_y: f32 },
    PanelResizeBy { delta: f32 },
    Focus(Region),
    FocusNext,
    FocusPrevious,
}

/// All mutable layout state. Kept independent of the app so geometry and
/// collapsed-state behavior can be unit tested without a renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellState {
    pub panel_height: f32,
    pub panel_visible: bool,
    pub context_visible: bool,
    pub sidebar_visible: bool,
    pub dragging_splitter: bool,
    pub active_panel: Panel,
    pub active_tab: usize,
    pub activity_selected: usize,
    pub focused: Region,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            panel_height: BOTTOM_PANEL_DEFAULT_HEIGHT,
            panel_visible: true,
            context_visible: true,
            sidebar_visible: true,
            dragging_splitter: false,
            active_panel: Panel::Terminal,
            active_tab: 0,
            activity_selected: 0,
            focused: Region::Center,
        }
    }
}

impl ShellState {
    /// Re-clamp after a window resize or a locale/font change.
    pub fn clamp_to(&mut self, window: Size) {
        self.panel_height = if self.panel_visible {
            clamp_panel_height(self.panel_height, window.height)
        } else {
            0.0
        };
    }

    pub fn workspace_height(&self, window: Size) -> f32 {
        split_heights(window.height, self.panel_height, self.panel_visible).0
    }

    pub fn panel_height(&self, window: Size) -> f32 {
        split_heights(window.height, self.panel_height, self.panel_visible).1
    }

    pub fn center_width(&self, window: Size) -> f32 {
        center_width(window.width, self.context_visible, self.sidebar_visible)
    }

    pub fn apply(&mut self, message: ShellMessage, window: Size) {
        match message {
            ShellMessage::ActivitySelected(index) => {
                self.activity_selected = index;
                self.focused = Region::ActivityRail;
            }
            ShellMessage::TabSelected(index) => {
                self.active_tab = index;
                self.focused = Region::Center;
            }
            ShellMessage::PanelSelected(panel) => {
                self.active_panel = panel;
                self.focused = Region::BottomPanel;
            }
            ShellMessage::TogglePanel => {
                self.panel_visible = !self.panel_visible;
                self.dragging_splitter = false;
                if self.panel_visible {
                    self.clamp_to(window);
                } else if self.focused == Region::BottomPanel {
                    self.focused = Region::Center;
                }
            }
            ShellMessage::ToggleContext => {
                self.context_visible = !self.context_visible;
                if !self.context_visible && self.focused == Region::ContextBar {
                    self.focused = Region::Center;
                }
            }
            ShellMessage::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
                if !self.sidebar_visible && self.focused == Region::PrimarySidebar {
                    self.focused = Region::Center;
                }
            }
            ShellMessage::SplitterPressed => {
                self.dragging_splitter = true;
                self.focused = Region::BottomPanel;
            }
            ShellMessage::SplitterReleased => self.dragging_splitter = false,
            ShellMessage::PanelResizeFromCursor { cursor_y } => {
                if self.dragging_splitter && self.panel_visible {
                    self.panel_height = panel_height_from_cursor(cursor_y, window.height);
                }
            }
            ShellMessage::PanelResizeBy { delta } => {
                if self.panel_visible {
                    self.panel_height =
                        clamp_panel_height(self.panel_height + delta, window.height);
                }
            }
            ShellMessage::Focus(region) => {
                if region.is_visible(self) {
                    self.focused = region;
                }
            }
            ShellMessage::FocusNext => self.move_focus(1),
            ShellMessage::FocusPrevious => self.move_focus(-1),
        }
    }

    fn move_focus(&mut self, step: isize) {
        let count = Region::ALL.len() as isize;
        let mut index = Region::ALL
            .iter()
            .position(|region| *region == self.focused)
            .unwrap_or(0) as isize;
        for _ in 0..count {
            index = (index + step).rem_euclid(count);
            let candidate = Region::ALL[index as usize];
            if candidate.is_visible(self) {
                self.focused = candidate;
                return;
            }
        }
    }
}

/// Content for each region, supplied by the app. Owned `Element`s keep the
/// shell generic over the app's `Message` type.
pub struct ShellParts<'a, M> {
    pub header: Element<'a, M>,
    pub activity: Element<'a, M>,
    pub sidebar: Element<'a, M>,
    pub center: Element<'a, M>,
    pub context: Element<'a, M>,
    pub panel: Element<'a, M>,
    pub status: Element<'a, M>,
}

fn region<'a, M: 'a>(
    tokens: Tokens,
    state: &ShellState,
    which: Region,
    content: Element<'a, M>,
    width: Length,
    height: Length,
) -> Element<'a, M> {
    let focused = state.focused == which;
    container(content)
        .width(width)
        .height(height)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(tokens.surface)),
            text_color: Some(tokens.text),
            // Width change plus color: focus is never signalled by color alone.
            border: Border {
                color: if focused {
                    tokens.accent
                } else {
                    tokens.border
                },
                width: if focused { 2.0 } else { 1.0 },
                radius: iced::border::Radius::new(0.0),
            },
            ..Default::default()
        })
        .into()
}

fn splitter<'a, M: Clone + 'a>(
    tokens: Tokens,
    on_shell: &impl Fn(ShellMessage) -> M,
) -> Element<'a, M> {
    let grab: MouseArea<'a, M> = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(SPLITTER_THICKNESS))
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(tokens.surface_alt)),
                border: Border {
                    color: tokens.border,
                    width: 1.0,
                    radius: iced::border::Radius::new(0.0),
                },
                ..Default::default()
            }),
    );
    grab.on_press(on_shell(ShellMessage::SplitterPressed))
        .on_release(on_shell(ShellMessage::SplitterReleased))
        .interaction(Interaction::ResizingVertically)
        .into()
}

/// Compose the full shell: header / status chrome plus the six regions.
pub fn shell<'a, M: Clone + 'a>(
    tokens: Tokens,
    state: &ShellState,
    window: Size,
    parts: ShellParts<'a, M>,
    on_shell: impl Fn(ShellMessage) -> M + 'a,
) -> Element<'a, M> {
    let workspace_height = state.workspace_height(window);
    let panel_height = state.panel_height(window);

    let mut columns: Vec<Element<'a, M>> = vec![region(
        tokens,
        state,
        Region::ActivityRail,
        parts.activity,
        Length::Fixed(ACTIVITY_RAIL_WIDTH),
        Length::Fill,
    )];
    if state.sidebar_visible {
        columns.push(region(
            tokens,
            state,
            Region::PrimarySidebar,
            parts.sidebar,
            Length::Fixed(SIDEBAR_WIDTH),
            Length::Fill,
        ));
    }

    let mut center_column: Vec<Element<'a, M>> = vec![region(
        tokens,
        state,
        Region::Center,
        parts.center,
        Length::Fill,
        Length::Fixed(workspace_height),
    )];
    if state.panel_visible && panel_height > 0.0 {
        center_column.push(splitter(tokens, &on_shell));
        center_column.push(region(
            tokens,
            state,
            Region::BottomPanel,
            parts.panel,
            Length::Fill,
            Length::Fixed(panel_height),
        ));
    }
    columns.push(
        column(center_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    );

    if state.context_visible {
        columns.push(region(
            tokens,
            state,
            Region::ContextBar,
            parts.context,
            Length::Fixed(CONTEXT_BAR_WIDTH),
            Length::Fill,
        ));
    }

    column![
        container(parts.header)
            .width(Length::Fill)
            .height(Length::Fixed(HEADER_HEIGHT))
            .style(panel_style(tokens)),
        row(columns).width(Length::Fill).height(Length::Fill),
        container(parts.status)
            .width(Length::Fill)
            .height(Length::Fixed(STATUS_HEIGHT))
            .style(panel_style(tokens)),
    ]
    .into()
}

/// Compact launchers for the activity rail. `entries` are short labels
/// derived from localized strings by the app.
pub fn activity_rail<'a, M: Clone + 'a>(
    tokens: Tokens,
    entries: &[String],
    selected: usize,
    on_select: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    let mut children: Vec<Element<'a, M>> = Vec::new();
    for (index, label) in entries.iter().enumerate() {
        let active = index == selected;
        let label = label.clone();
        children.push(
            button(text(label).size(BODY_SIZE))
                .width(Length::Fixed(ACTIVITY_RAIL_WIDTH - SPACE_XS * 2.0))
                .height(Length::Fixed(REGION_BUTTON_HEIGHT))
                .on_press(on_select(index))
                .style(move |_, _| iced::widget::button::Style {
                    background: Some(Background::Color(if active {
                        tokens.accent
                    } else {
                        tokens.surface_alt
                    })),
                    text_color: if active {
                        tokens.background
                    } else {
                        tokens.text
                    },
                    border: Border {
                        color: if active { tokens.accent } else { tokens.border },
                        width: if active { 2.0 } else { 1.0 },
                        radius: iced::border::Radius::new(SPACE_SM),
                    },
                    ..Default::default()
                })
                .into(),
        );
    }
    container(
        Column::with_children(children)
            .spacing(SPACE_XS)
            .padding(SPACE_XS),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(panel_style(tokens))
    .into()
}

/// Tab-bar primitive shared by the workspace area and the bottom panel.
pub fn tab_bar<'a, M: Clone + 'a>(
    tokens: Tokens,
    tabs: &[String],
    active: usize,
    on_select: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    let mut children: Vec<Element<'a, M>> = Vec::new();
    for (index, label) in tabs.iter().enumerate() {
        let selected = index == active;
        let label = label.clone();
        children.push(
            button(text(label).size(BODY_SIZE))
                .height(Length::Fixed(TAB_BAR_HEIGHT - SPACE_XS * 2.0))
                .on_press(on_select(index))
                .style(move |_, _| iced::widget::button::Style {
                    background: Some(Background::Color(if selected {
                        tokens.accent
                    } else {
                        tokens.surface_alt
                    })),
                    text_color: if selected {
                        tokens.background
                    } else {
                        tokens.text
                    },
                    border: Border {
                        color: if selected {
                            tokens.accent
                        } else {
                            tokens.border
                        },
                        width: if selected { 2.0 } else { 1.0 },
                        radius: iced::border::Radius::new(SPACE_SM),
                    },
                    ..Default::default()
                })
                .into(),
        );
    }
    container(row(children).spacing(SPACE_XS).padding(SPACE_XS))
        .width(Length::Fill)
        .height(Length::Fixed(TAB_BAR_HEIGHT))
        .style(panel_style(tokens))
        .into()
}

/// Bottom panel: localized page tabs above caller-supplied placeholder body.
pub fn bottom_panel<'a, M: Clone + 'a>(
    tokens: Tokens,
    active: Panel,
    labels: &[String],
    content: Element<'a, M>,
    on_select: impl Fn(Panel) -> M + 'a,
) -> Element<'a, M> {
    let tabs = column![
        tab_bar(tokens, labels, active.index(), move |index| on_select(
            Panel::from_index(index)
        )),
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(SPACE_SM),
    ];
    tabs.into()
}

/// Optional right context bar.
pub fn context_bar<'a, M: 'a>(tokens: Tokens, title: String, lines: &[String]) -> Element<'a, M> {
    let mut children: Vec<Element<'a, M>> = vec![text(title).size(BODY_SIZE).into()];
    for line in lines {
        children.push(text(line.clone()).size(BODY_SIZE - 2.0).into());
    }
    container(
        Column::with_children(children)
            .spacing(SPACE_SM)
            .padding(SPACE_MD),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(panel_style(tokens))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> Size {
        Size::new(1200.0, 800.0)
    }

    #[test]
    fn split_area_reserves_header_and_status() {
        assert_eq!(
            split_area_height(800.0),
            800.0 - HEADER_HEIGHT - STATUS_HEIGHT
        );
        assert_eq!(split_area_height(HEADER_HEIGHT + STATUS_HEIGHT), 0.0);
    }

    #[test]
    fn default_geometry_at_1200x800() {
        let state = ShellState::default();
        let (workspace, panel) = split_heights(800.0, state.panel_height, true);
        assert_eq!(panel, BOTTOM_PANEL_DEFAULT_HEIGHT);
        assert_eq!(
            workspace,
            split_area_height(800.0) - SPLITTER_THICKNESS - panel
        );
        assert_eq!(
            state.center_width(window()),
            1200.0 - ACTIVITY_RAIL_WIDTH - SIDEBAR_WIDTH - CONTEXT_BAR_WIDTH
        );
    }

    #[test]
    fn hidden_panel_gives_entire_split_area_to_workspace() {
        assert_eq!(
            split_heights(800.0, 240.0, false),
            (split_area_height(800.0), 0.0)
        );
        let mut state = ShellState::default();
        state.apply(ShellMessage::TogglePanel, window());
        assert!(!state.panel_visible);
        assert_eq!(state.panel_height(window()), 0.0);
        assert_eq!(state.workspace_height(window()), split_area_height(800.0));
    }

    #[test]
    fn panel_height_is_clamped_at_both_ends() {
        assert_eq!(clamp_panel_height(1.0, 800.0), BOTTOM_PANEL_MIN_HEIGHT);
        let max = split_area_height(800.0) - SPLITTER_THICKNESS - MIN_WORKSPACE_HEIGHT;
        assert_eq!(clamp_panel_height(100_000.0, 800.0), max);
        assert_eq!(
            clamp_panel_height(240.0, HEADER_HEIGHT + STATUS_HEIGHT),
            0.0
        );
    }

    #[test]
    fn drag_maps_cursor_to_clamped_panel_height() {
        let split_bottom = HEADER_HEIGHT + split_area_height(800.0);
        assert_eq!(
            panel_height_from_cursor(split_bottom - 240.0, 800.0),
            BOTTOM_PANEL_DEFAULT_HEIGHT
        );
        assert_eq!(
            panel_height_from_cursor(split_bottom - 1.0, 800.0),
            BOTTOM_PANEL_MIN_HEIGHT
        );
    }

    #[test]
    fn keyboard_resize_and_toggle_round_trip() {
        let mut state = ShellState::default();
        state.apply(ShellMessage::PanelResizeBy { delta: 60.0 }, window());
        assert_eq!(state.panel_height, 300.0);
        state.apply(ShellMessage::PanelResizeBy { delta: -1_000.0 }, window());
        assert_eq!(state.panel_height, BOTTOM_PANEL_MIN_HEIGHT);
        state.apply(ShellMessage::TogglePanel, window());
        assert!(!state.panel_visible);
        state.apply(ShellMessage::PanelResizeBy { delta: 20.0 }, window());
        assert_eq!(state.panel_height, BOTTOM_PANEL_MIN_HEIGHT);
        state.apply(ShellMessage::TogglePanel, window());
        assert!(state.panel_visible);
        assert_eq!(state.panel_height, BOTTOM_PANEL_MIN_HEIGHT);
    }

    #[test]
    fn context_bar_can_be_hidden_and_restores_center_width() {
        let mut state = ShellState::default();
        let narrow = state.center_width(window());
        state.apply(ShellMessage::ToggleContext, window());
        assert!(!state.context_visible);
        assert_eq!(state.center_width(window()), narrow + CONTEXT_BAR_WIDTH);
    }

    #[test]
    fn focus_navigation_skips_hidden_regions() {
        let mut state = ShellState::default();
        state.apply(ShellMessage::ToggleContext, window());
        state.apply(ShellMessage::TogglePanel, window());
        state.apply(ShellMessage::Focus(Region::Center), window());
        state.apply(ShellMessage::FocusNext, window());
        assert_eq!(state.focused, Region::StatusBar);
        state.apply(ShellMessage::FocusNext, window());
        assert_eq!(state.focused, Region::ActivityRail);
        state.apply(ShellMessage::FocusPrevious, window());
        assert_eq!(state.focused, Region::StatusBar);
    }

    #[test]
    fn hiding_a_focused_region_moves_focus_to_center() {
        let mut state = ShellState::default();
        state.apply(ShellMessage::Focus(Region::BottomPanel), window());
        state.apply(ShellMessage::TogglePanel, window());
        assert_eq!(state.focused, Region::Center);
        state.apply(ShellMessage::Focus(Region::ContextBar), window());
        state.apply(ShellMessage::ToggleContext, window());
        assert_eq!(state.focused, Region::Center);
    }

    #[test]
    fn splitter_drag_only_applies_while_pressed() {
        let mut state = ShellState::default();
        state.apply(
            ShellMessage::PanelResizeFromCursor { cursor_y: 400.0 },
            window(),
        );
        assert_eq!(state.panel_height, BOTTOM_PANEL_DEFAULT_HEIGHT);
        state.apply(ShellMessage::SplitterPressed, window());
        assert!(state.dragging_splitter);
        state.apply(
            ShellMessage::PanelResizeFromCursor { cursor_y: 400.0 },
            window(),
        );
        assert_ne!(state.panel_height, BOTTOM_PANEL_DEFAULT_HEIGHT);
        state.apply(ShellMessage::SplitterReleased, window());
        let locked = state.panel_height;
        state.apply(
            ShellMessage::PanelResizeFromCursor { cursor_y: 700.0 },
            window(),
        );
        assert_eq!(state.panel_height, locked);
    }

    #[test]
    fn panel_tabs_cycle_in_order() {
        assert_eq!(Panel::Terminal.next(), Panel::Problems);
        assert_eq!(Panel::Logs.next(), Panel::Terminal);
        let mut state = ShellState::default();
        state.apply(ShellMessage::PanelSelected(Panel::Output), window());
        assert_eq!(state.active_panel, Panel::Output);
        assert_eq!(state.focused, Region::BottomPanel);
    }

    #[test]
    fn tiny_window_has_no_room_for_a_panel() {
        let tiny = Size::new(320.0, HEADER_HEIGHT + STATUS_HEIGHT + 100.0);
        let mut state = ShellState::default();
        state.clamp_to(tiny);
        assert_eq!(state.panel_height, 0.0);
        assert_eq!(state.workspace_height(tiny), split_area_height(tiny.height));
    }
}
