//! ANSI terminal grid with a hard scrollback cap.
//!
//! PTY bytes are parsed with `vte`; printable cells and the common cursor/erase
//! controls are applied to a line buffer. The buffer holds at most
//! `rows + scrollback_lines` lines, so a 100k-line burst cannot grow memory
//! without bound.

use std::collections::VecDeque;

use vte::{Params, Parser, Perform};

pub const DEFAULT_COLS: u16 = 80;
pub const DEFAULT_ROWS: u16 = 24;
pub const DEFAULT_SCROLLBACK_LINES: u32 = 10_000;

pub struct Grid {
    cols: usize,
    rows: usize,
    scrollback: usize,
    capacity: usize,
    lines: VecDeque<Vec<char>>,
    cursor_row: usize,
    cursor_col: usize,
    parser: Parser,
}

impl Grid {
    /// `scrollback_lines` is additional history beyond the visible `rows`.
    pub fn new(cols: u16, rows: u16, scrollback_lines: u32) -> Self {
        let cols = usize::from(cols.max(1));
        let rows = usize::from(rows.max(1));
        let scrollback = usize::try_from(scrollback_lines).unwrap_or(usize::MAX / 2);
        let capacity = rows.saturating_add(scrollback);
        Self {
            cols,
            rows,
            scrollback,
            capacity,
            lines: VecDeque::from(vec![Vec::new()]),
            cursor_row: 0,
            cursor_col: 0,
            parser: Parser::new(),
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of stored lines, including the visible rows. Never exceeds
    /// `rows + scrollback_lines`.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Feed raw (already redacted) PTY bytes into the parser.
    pub fn advance(&mut self, bytes: &[u8]) {
        // Take the parser out so the mutable borrow of `self` for the
        // `Perform` implementation does not overlap with `self.parser`.
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        parser.advance(self, bytes);
        self.parser = parser;
    }

    /// Last `max_lines` lines, oldest first.
    pub fn snapshot(&self, max_lines: usize) -> Vec<String> {
        let take = max_lines.min(self.lines.len());
        self.lines
            .iter()
            .rev()
            .take(take)
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Replace the viewport size. Existing history is retained; the cursor is
    /// clamped back into the visible rows.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = usize::from(cols.max(1));
        self.rows = usize::from(rows.max(1));
        self.capacity = self.rows.saturating_add(self.scrollback);
        self.cursor_col = self.cursor_col.min(self.cols - 1);
        self.cursor_row = self.cursor_row.min(self.lines.len().saturating_sub(1));
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.linefeed();
        }
        let line = &mut self.lines[self.cursor_row];
        if line.len() <= self.cursor_col {
            line.resize(self.cursor_col + 1, ' ');
        }
        line[self.cursor_col] = ch;
        self.cursor_col += 1;
    }

    fn linefeed(&mut self) {
        if self.cursor_row + 1 >= self.lines.len() {
            self.lines.push_back(Vec::new());
            while self.lines.len() > self.capacity {
                self.lines.pop_front();
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
            }
            self.cursor_row = self.lines.len() - 1;
        } else {
            self.cursor_row += 1;
        }
    }

    fn erase_line(&mut self, mode: usize) {
        let line = &mut self.lines[self.cursor_row];
        match mode {
            1 => {
                for cell in line.iter_mut().take(self.cursor_col) {
                    *cell = ' ';
                }
            }
            2 => line.clear(),
            _ => line.truncate(self.cursor_col),
        }
    }

    fn erase_display(&mut self, mode: usize) {
        match mode {
            2 => {
                for line in self.lines.iter_mut() {
                    line.clear();
                }
                self.cursor_col = 0;
            }
            1 => {
                for line in self.lines.iter_mut().take(self.cursor_row + 1) {
                    for cell in line.iter_mut().take(self.cursor_col + 1) {
                        *cell = ' ';
                    }
                }
            }
            _ => {
                self.erase_line(0);
                for line in self.lines.iter_mut().skip(self.cursor_row + 1) {
                    line.clear();
                }
            }
        }
    }
}

impl Perform for Grid {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => self.linefeed(),
            b'\r' => self.cursor_col = 0,
            b'\x08' => self.cursor_col = self.cursor_col.saturating_sub(1),
            b'\t' => {
                self.cursor_col = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = self.cursor_col.min(self.cols - 1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore || !intermediates.is_empty() {
            return;
        }
        let params: Vec<usize> = params
            .iter()
            .map(|p| p.first().copied().unwrap_or(0) as usize)
            .collect();
        let arg = |index: usize, default: usize| -> usize {
            params
                .get(index)
                .copied()
                .filter(|v| *v != 0)
                .unwrap_or(default)
        };
        match action {
            'A' => self.cursor_row = self.cursor_row.saturating_sub(arg(0, 1)),
            'B' => self.cursor_row = (self.cursor_row + arg(0, 1)).min(self.lines.len() - 1),
            'C' => self.cursor_col = (self.cursor_col + arg(0, 1)).min(self.cols - 1),
            'D' => self.cursor_col = self.cursor_col.saturating_sub(arg(0, 1)),
            'G' => self.cursor_col = arg(0, 1).saturating_sub(1).min(self.cols - 1),
            'H' | 'f' => {
                self.cursor_row = arg(0, 1)
                    .saturating_sub(1)
                    .min(self.lines.len().saturating_sub(1));
                self.cursor_col = arg(1, 1).saturating_sub(1).min(self.cols - 1);
            }
            'J' => self.erase_display(arg(0, 0)),
            'K' => self.erase_line(arg(0, 0)),
            // Colour/style (SGR) and mode changes carry no cell text.
            _ => {}
        }
    }

    // OSC/DCS payloads (titles, clipboard, keyboard protocols) never draw cells.
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_newlines_and_carriage_returns() {
        let mut grid = Grid::new(80, 24, 10);
        grid.advance(b"hello\r\nworld");
        assert_eq!(grid.snapshot(2), vec!["hello", "world"]);
        assert!(grid.snapshot(1)[0].ends_with("world"));
    }

    #[test]
    fn applies_cursor_and_erase_controls() {
        let mut grid = Grid::new(80, 24, 10);
        grid.advance(b"abc\x1b[3DZ");
        assert_eq!(grid.snapshot(1), vec!["Zbc"]);
        grid.advance(b"\r\x1b[2KXY");
        assert_eq!(grid.snapshot(1), vec!["XY"]);
    }

    #[test]
    fn long_lines_wrap_within_the_column_count() {
        let mut grid = Grid::new(10, 2, 2);
        let burst = "a".repeat(500);
        grid.advance(burst.as_bytes());
        for line in grid.snapshot(usize::MAX) {
            assert!(line.len() <= 10, "line grew past the grid width");
        }
    }

    #[test]
    fn hundred_k_line_burst_cannot_exceed_the_scrollback_cap() {
        let mut grid = Grid::new(80, 2, 8);
        let cap = grid.capacity();
        assert_eq!(cap, 10);
        let mut burst = "x\n".repeat(100_000);
        burst.push('x');
        grid.advance(burst.as_bytes());
        assert!(grid.len() <= cap, "stored {} lines, cap {cap}", grid.len());
        assert_eq!(grid.len(), cap);
        assert_eq!(grid.snapshot(1), vec!["x"]);
    }
}
