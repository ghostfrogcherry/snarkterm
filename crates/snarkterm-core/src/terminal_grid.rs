#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RgbColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl RgbColor {
    pub const DEFAULT_FG: Self = Self {
        r: 0.0,
        g: 0.96,
        b: 0.83,
    };

    pub fn as_array(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub ch: char,
    pub fg: RgbColor,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: RgbColor::DEFAULT_FG,
        }
    }
}

#[derive(Debug)]
pub struct TerminalBuffer {
    cells: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    rows: usize,
    cols: usize,
    current_fg: RgbColor,
}

impl TerminalBuffer {
    pub fn new(cols: usize, rows: usize) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        Self {
            cells: vec![vec![Cell::default(); cols]; rows],
            cursor_row: 0,
            cursor_col: 0,
            rows,
            cols,
            current_fg: RgbColor::DEFAULT_FG,
        }
    }

    pub fn put_char(&mut self, ch: char) {
        if ch == '\n' {
            self.newline();
            return;
        }
        if ch == '\r' {
            self.cursor_col = 0;
            return;
        }
        if ch == '\u{8}' || ch == '\u{7f}' {
            self.backspace();
            return;
        }
        if ch.is_control() {
            return;
        }

        if self.cursor_col >= self.cols {
            self.newline();
        }
        self.cells[self.cursor_row][self.cursor_col] = Cell {
            ch,
            fg: self.current_fg,
        };
        self.cursor_col += 1;
    }

    pub fn newline(&mut self) {
        self.cursor_col = 0;
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cursor_row += 1;
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_col == 0 {
            return;
        }
        self.cursor_col -= 1;
        self.cells[self.cursor_row][self.cursor_col] = Cell::default();
    }

    pub fn move_cursor(&mut self, row: usize, col: usize) {
        self.cursor_row = row.min(self.rows.saturating_sub(1));
        self.cursor_col = col.min(self.cols.saturating_sub(1));
    }

    pub fn move_relative(&mut self, row_delta: isize, col_delta: isize) {
        let row = self.cursor_row.saturating_add_signed(row_delta);
        let col = self.cursor_col.saturating_add_signed(col_delta);
        self.move_cursor(row, col);
    }

    pub fn clear_screen(&mut self) {
        for row in &mut self.cells {
            row.fill(Cell::default());
        }
        self.move_cursor(0, 0);
    }

    pub fn clear_line_from_cursor(&mut self) {
        for col in self.cursor_col..self.cols {
            self.cells[self.cursor_row][col] = Cell::default();
        }
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    pub fn cells(&self) -> &[Vec<Cell>] {
        &self.cells
    }

    fn scroll_up(&mut self) {
        self.cells.remove(0);
        self.cells.push(vec![Cell::default(); self.cols]);
    }

    fn reset_graphics(&mut self) {
        self.current_fg = RgbColor::DEFAULT_FG;
    }

    fn set_ansi_fg(&mut self, color: i32) {
        self.current_fg = match color {
            30 => RgbColor {
                r: 0.20,
                g: 0.22,
                b: 0.28,
            },
            31 => RgbColor {
                r: 1.00,
                g: 0.23,
                b: 0.42,
            },
            32 => RgbColor {
                r: 0.54,
                g: 1.00,
                b: 0.50,
            },
            33 => RgbColor {
                r: 1.00,
                g: 0.72,
                b: 0.42,
            },
            34 => RgbColor {
                r: 0.38,
                g: 0.62,
                b: 1.00,
            },
            35 => RgbColor {
                r: 0.86,
                g: 0.45,
                b: 1.00,
            },
            36 => RgbColor {
                r: 0.00,
                g: 0.96,
                b: 0.83,
            },
            37 => RgbColor {
                r: 0.85,
                g: 0.87,
                b: 0.91,
            },
            90 => RgbColor {
                r: 0.43,
                g: 0.46,
                b: 0.54,
            },
            91 => RgbColor {
                r: 1.00,
                g: 0.35,
                b: 0.50,
            },
            92 => RgbColor {
                r: 0.65,
                g: 1.00,
                b: 0.61,
            },
            93 => RgbColor {
                r: 1.00,
                g: 0.82,
                b: 0.50,
            },
            94 => RgbColor {
                r: 0.54,
                g: 0.72,
                b: 1.00,
            },
            95 => RgbColor {
                r: 0.94,
                g: 0.58,
                b: 1.00,
            },
            96 => RgbColor {
                r: 0.31,
                g: 1.00,
                b: 0.88,
            },
            97 => RgbColor {
                r: 1.00,
                g: 1.00,
                b: 1.00,
            },
            _ => self.current_fg,
        };
    }
}

#[derive(Default)]
pub struct OutputParser {
    state: ParserState,
    csi: String,
}

impl OutputParser {
    pub fn feed(&mut self, bytes: &[u8], terminal: &mut TerminalBuffer) {
        for byte in bytes {
            let ch = *byte as char;
            match self.state {
                ParserState::Ground => {
                    if ch == '\x1b' {
                        self.state = ParserState::Escape;
                    } else {
                        terminal.put_char(ch);
                    }
                }
                ParserState::Escape => {
                    if ch == '[' {
                        self.csi.clear();
                        self.state = ParserState::Csi;
                    } else if ch == ']' {
                        self.state = ParserState::Osc;
                    } else if ch == 'c' {
                        terminal.clear_screen();
                        self.state = ParserState::Ground;
                    } else {
                        self.state = ParserState::Ground;
                    }
                }
                ParserState::Csi => {
                    if ch.is_ascii_alphabetic() || ch == '~' {
                        self.apply_csi(ch, terminal);
                        self.state = ParserState::Ground;
                    } else if self.csi.len() < 64 {
                        self.csi.push(ch);
                    }
                }
                ParserState::Osc => {
                    if ch == '\u{7}' {
                        self.state = ParserState::Ground;
                    } else if ch == '\x1b' {
                        self.state = ParserState::OscEscape;
                    }
                }
                ParserState::OscEscape => {
                    self.state = ParserState::Ground;
                }
            }
        }
    }

    fn apply_csi(&self, command: char, terminal: &mut TerminalBuffer) {
        let params = parse_csi_params(&self.csi);
        let first = params.first().copied().unwrap_or(1).max(1) as usize;

        match command {
            'A' => terminal.move_relative(-(first as isize), 0),
            'B' => terminal.move_relative(first as isize, 0),
            'C' => terminal.move_relative(0, first as isize),
            'D' => terminal.move_relative(0, -(first as isize)),
            'G' => terminal.move_cursor(terminal.cursor_row, first.saturating_sub(1)),
            'H' | 'f' => {
                let row = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = params.get(1).copied().unwrap_or(1).max(1) as usize - 1;
                terminal.move_cursor(row, col);
            }
            'J' => {
                if params.first().copied().unwrap_or(0) == 2 {
                    terminal.clear_screen();
                }
            }
            'K' => terminal.clear_line_from_cursor(),
            'm' => apply_sgr(&params, terminal),
            _ => {}
        }
    }
}

#[derive(Default)]
enum ParserState {
    #[default]
    Ground,
    Escape,
    Csi,
    Osc,
    OscEscape,
}

fn apply_sgr(params: &[i32], terminal: &mut TerminalBuffer) {
    if params.is_empty() {
        terminal.reset_graphics();
        return;
    }

    let mut iter = params.iter().copied().peekable();
    while let Some(param) = iter.next() {
        match param {
            0 => terminal.reset_graphics(),
            30..=37 | 90..=97 => terminal.set_ansi_fg(param),
            39 => terminal.current_fg = RgbColor::DEFAULT_FG,
            38 => {
                if iter.next() == Some(2) {
                    let (Some(r), Some(g), Some(b)) = (iter.next(), iter.next(), iter.next())
                    else {
                        continue;
                    };
                    terminal.current_fg = RgbColor {
                        r: (r.clamp(0, 255) as f32) / 255.0,
                        g: (g.clamp(0, 255) as f32) / 255.0,
                        b: (b.clamp(0, 255) as f32) / 255.0,
                    };
                }
            }
            _ => {}
        }
    }
}

fn parse_csi_params(raw: &str) -> Vec<i32> {
    raw.trim_start_matches('?')
        .split(';')
        .filter_map(|part| {
            if part.is_empty() {
                Some(0)
            } else {
                part.parse().ok()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{OutputParser, RgbColor, TerminalBuffer};

    #[test]
    fn terminal_buffer_tracks_new_lines() {
        let mut terminal = TerminalBuffer::new(80, 3);

        for ch in "hello\nworld".chars() {
            terminal.put_char(ch);
        }

        assert_eq!(terminal.visible_lines()[0], "hello");
        assert_eq!(terminal.visible_lines()[1], "world");
    }

    #[test]
    fn parser_applies_cursor_positioning() {
        let mut terminal = TerminalBuffer::new(10, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"abc\x1b[2;3HZ", &mut terminal);

        assert_eq!(terminal.visible_lines()[0], "abc");
        assert_eq!(terminal.visible_lines()[1], "  Z");
    }

    #[test]
    fn parser_clears_screen() {
        let mut terminal = TerminalBuffer::new(10, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"abc\x1b[2JZ", &mut terminal);

        assert_eq!(terminal.visible_lines()[0], "Z");
    }

    #[test]
    fn parser_skips_osc_sequences() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"a\x1b]0;title\x07b", &mut terminal);

        assert_eq!(terminal.visible_lines()[0], "ab");
    }

    #[test]
    fn parser_applies_sgr_foreground_color() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[31mR\x1b[0mD", &mut terminal);

        assert_eq!(terminal.cells()[0][0].ch, 'R');
        assert_eq!(
            terminal.cells()[0][0].fg,
            RgbColor {
                r: 1.0,
                g: 0.23,
                b: 0.42
            }
        );
        assert_eq!(terminal.cells()[0][1].fg, RgbColor::DEFAULT_FG);
    }

    #[test]
    fn parser_applies_truecolor_foreground() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[38;2;255;128;0mX", &mut terminal);

        assert_eq!(
            terminal.cells()[0][0].fg,
            RgbColor {
                r: 1.0,
                g: 128.0 / 255.0,
                b: 0.0
            }
        );
    }
}
