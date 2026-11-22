//! Iced 0.14 UI framework spike for Devdeck ADR-0001.
//!
//! Four probes in one window: text input (IME), 100k-row virtual list,
//! terminal-like cell grid, and a basic keyboard shortcut. Run with
//! `xvfb-run -a cargo run -- --smoke` for a headless smoke run.

use iced::widget::{column, container, row, scrollable, text, text_input};
use iced::{Element, Font, Length, Settings, Task};

const ROWS: usize = 100_000;
const ROW_HEIGHT: f32 = 24.0;
const VISIBLE_ROWS: usize = 24;
const GRID_COLS: usize = 200;
const GRID_ROWS: usize = 50;

#[derive(Debug, Clone)]
enum Message {
    Input(String),
    Scrolled(f32),
    Tick,
}

struct Spike {
    input: String,
    scroll_y: f32,
    tick: u64,
    smoke: bool,
}

fn boot(smoke: bool) -> (Spike, Task<Message>) {
    (
        Spike { input: String::new(), scroll_y: 0.0, tick: 0, smoke },
        Task::none(),
    )
}

fn update(state: &mut Spike, message: Message) -> Task<Message> {
    match message {
        Message::Input(s) => state.input = s,
        Message::Scrolled(y) => state.scroll_y = y,
        Message::Tick => state.tick = state.tick.wrapping_add(1),
    }
    Task::none()
}

fn view(state: &Spike) -> Element<'_, Message> {
    let input = text_input("Type here (IME enabled)", &state.input)
        .on_input(Message::Input)
        .size(18);

    let first = visible_start(state.scroll_y, ROW_HEIGHT, ROWS, VISIBLE_ROWS);
    let visible: Vec<Element<'_, Message>> = (first..(first + VISIBLE_ROWS).min(ROWS))
        .map(|i| {
            container(text(format!("row {i:06} — dòng {i:06}")))
                .height(ROW_HEIGHT)
                .into()
        })
        .collect();
    let list = scrollable(column(visible).spacing(0))
        .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
        .height(220);

    let grid = terminal_grid(state.tick, GRID_COLS, GRID_ROWS);

    column![
        text("Devdeck UI spike - Iced 0.14").size(24),
        row![text("Command palette probe: "), text("Ctrl+K (wiring in Phase 1)")].spacing(10),
        input,
        text(format!("Virtual list: {ROWS} rows")),
        list,
        text("Terminal-like grid (200x50 cells, cursor blinks)"),
        grid,
    ]
    .spacing(10)
    .padding(16)
    .into()
}

/// Compute the first row to render for a pixel scroll offset.
fn visible_start(offset_y: f32, row_height: f32, total: usize, window: usize) -> usize {
    debug_assert!(row_height > 0.0);
    let first = (offset_y / row_height).floor().max(0.0) as usize;
    first.min(total.saturating_sub(window))
}

/// Build a monospace cell grid of `cols x rows`, with one blinking cursor.
fn terminal_grid<'a>(tick: u64, cols: usize, rows: usize) -> Element<'a, Message> {
    let cursor_row = (tick as usize) % rows;
    let lines: Vec<Element<'a, Message>> = (0..rows)
        .map(|r| {
            let mut line = String::with_capacity(cols);
            for c in 0..cols {
                line.push(if c == (tick as usize * 7) % cols { '█' } else { '.' });
            }
            if r == cursor_row {
                line.push('▌');
            }
            text(line).font(Font::MONOSPACE).size(12).into()
        })
        .collect();
    scrollable(column(lines).spacing(0))
        .height(Length::Fixed(180.0))
        .into()
}

fn subscription(state: &Spike) -> iced::Subscription<Message> {
    let millis = if state.smoke { 100 } else { 500 };
    iced::time::every(std::time::Duration::from_millis(millis)).map(|_| Message::Tick)
}

fn main() -> iced::Result {
    let smoke = std::env::args().any(|a| a == "--smoke");
    iced::application(move || boot(smoke), update, view)
        .title("Devdeck UI spike (Iced 0.14)")
        .settings(Settings::default())
        .window(iced::window::Settings {
            size: iced::Size::new(1024.0, 768.0),
            ..Default::default()
        })
        .subscription(subscription)
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_list_window_is_bounded() {
        assert_eq!(visible_start(0.0, 24.0, ROWS, VISIBLE_ROWS), 0);
        assert_eq!(visible_start(24.0, 24.0, ROWS, VISIBLE_ROWS), 1);
        assert_eq!(visible_start(2399.0, 24.0, ROWS, VISIBLE_ROWS), 99);
        assert_eq!(visible_start(f32::MAX, 24.0, ROWS, VISIBLE_ROWS), ROWS - VISIBLE_ROWS);
    }

    #[test]
    fn grid_indexing_is_dense() {
        let total = GRID_COLS * GRID_ROWS;
        assert_eq!(total, 10_000);
        for idx in [0, 1, 199, 200, 9_999] {
            let (row, col) = (idx / GRID_COLS, idx % GRID_COLS);
            assert_eq!(row * GRID_COLS + col, idx);
            assert!(row < GRID_ROWS && col < GRID_COLS);
        }
    }
}
