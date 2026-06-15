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

    pub fn scale(self, factor: f32) -> Self {
        Self {
            r: (self.r * factor).clamp(0.0, 1.0),
            g: (self.g * factor).clamp(0.0, 1.0),
            b: (self.b * factor).clamp(0.0, 1.0),
        }
    }

    pub fn as_array(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub ch: char,
    pub fg: RgbColor,
    pub bg: Option<RgbColor>,
    pub underline: bool,
    pub italic: bool,
    pub reverse: bool,
    pub strikethrough: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: RgbColor::DEFAULT_FG,
            bg: None,
            underline: false,
            italic: false,
            reverse: false,
            strikethrough: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GraphicsState {
    fg: RgbColor,
    bg: Option<RgbColor>,
    intensity: Intensity,
    underline: bool,
    italic: bool,
    reverse: bool,
    strikethrough: bool,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            fg: RgbColor::DEFAULT_FG,
            bg: None,
            intensity: Intensity::Normal,
            underline: false,
            italic: false,
            reverse: false,
            strikethrough: false,
        }
    }
}

impl GraphicsState {
    fn effective_fg(self) -> RgbColor {
        let base = match self.intensity {
            Intensity::Normal => self.fg,
            Intensity::Bold => self.fg.scale(1.25),
            Intensity::Dim => self.fg.scale(0.55),
        };
        if self.reverse {
            self.unwrap_bg().scale(base.r * 0.3 + base.g * 0.6 + base.b * 0.1)
        } else {
            base
        }
    }

    fn effective_bg(self) -> Option<RgbColor> {
        let base = self.bg;
        if self.reverse {
            Some(self.fg)
        } else {
            base
        }
    }

    fn unwrap_bg(self) -> RgbColor {
        self.bg.unwrap_or(RgbColor {
            r: 0.031,
            g: 0.039,
            b: 0.059,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum Intensity {
    Normal,
    Bold,
    Dim,
}

#[derive(Debug)]
pub struct TerminalBuffer {
    cells: Vec<Vec<Cell>>,
    scrollback: Vec<Vec<Cell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    rows: usize,
    cols: usize,
    graphics: GraphicsState,
    pub mode_flags: u32,
}

impl TerminalBuffer {
    pub fn new(cols: usize, rows: usize) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        Self {
            cells: vec![vec![Cell::default(); cols]; rows],
            scrollback: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            rows,
            cols,
            graphics: GraphicsState::default(),
            mode_flags: 0,
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
            fg: self.graphics.effective_fg(),
            bg: self.graphics.effective_bg(),
            underline: self.graphics.underline,
            italic: self.graphics.italic,
            reverse: self.graphics.reverse,
            strikethrough: self.graphics.strikethrough,
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

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let rows = rows.max(1);
        let cols = cols.max(1);
        let mut new_cells = vec![vec![Cell::default(); cols]; rows];
        let copy_rows = self.rows.min(rows);
        let copy_cols = self.cols.min(cols);

        for (row, new_row) in new_cells.iter_mut().enumerate().take(copy_rows) {
            new_row[..copy_cols].copy_from_slice(&self.cells[row][..copy_cols]);
        }

        self.cells = new_cells;
        self.rows = rows;
        self.cols = cols;
        self.move_cursor(self.cursor_row, self.cursor_col);
    }

    pub fn scroll_up(&mut self) {
        let row = self.cells.remove(0);
        self.scrollback.push(row);
        if self.scrollback.len() > 10000 {
            self.scrollback.remove(0);
        }
        self.cells.push(vec![Cell::default(); self.cols]);
    }

    pub fn scroll_page_up(&mut self) -> usize {
        let page_size = self.rows;
        let max_offset = self.scrollback.len().saturating_sub(page_size);
        if self.scroll_offset < max_offset {
            self.scroll_offset = (self.scroll_offset + page_size).min(max_offset);
        }
        self.scroll_offset
    }

    pub fn scroll_page_down(&mut self) -> usize {
        let page_size = self.rows;
        if self.scroll_offset > page_size {
            self.scroll_offset -= page_size;
        } else {
            self.scroll_offset = 0;
        }
        self.scroll_offset
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn display_cells(&self) -> Vec<Vec<Cell>> {
        if self.scroll_offset == 0 {
            return self.cells.clone();
        }
        let mut result = Vec::new();
        let start = self.scrollback.len().saturating_sub(self.scroll_offset);
        let end = (start + self.rows).min(self.scrollback.len());
        for row in &self.scrollback[start..end] {
            result.push(row.clone());
        }
        let remaining = self.rows.saturating_sub(result.len());
        for _ in 0..remaining {
            result.push(vec![Cell::default(); self.cols]);
        }
        result
    }

    pub fn set_mode(&mut self, mode: u32, enabled: bool) {
        if enabled {
            self.mode_flags |= mode;
        } else {
            self.mode_flags &= !mode;
        }
    }

    fn reset_graphics(&mut self) {
        self.graphics = GraphicsState::default();
    }

    fn set_ansi_fg(&mut self, color: i32) {
        if let Some(color) = ansi_color(color) {
            self.graphics.fg = color;
        }
    }

    fn set_ansi_bg(&mut self, color: i32) {
        self.graphics.bg = ansi_color(color - 10);
    }
}

fn ansi_color(color: i32) -> Option<RgbColor> {
    Some(match color {
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
        _ => return None,
    })
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
            'E' => {
                terminal.move_cursor(terminal.cursor_row, 0);
                terminal.move_relative(first as isize, 0);
            }
            'F' => {
                terminal.move_cursor(terminal.cursor_row, 0);
                terminal.move_relative(-(first as isize), 0);
            }
            'G' => terminal.move_cursor(terminal.cursor_row, first.saturating_sub(1)),
            'H' | 'f' => {
                let row = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = params.get(1).copied().unwrap_or(1).max(1) as usize - 1;
                terminal.move_cursor(row, col);
            }
            'J' => match params.first().copied().unwrap_or(0) {
                0 => terminal.clear_line_from_cursor(),
                2 => terminal.clear_screen(),
                _ => {}
            },
            'K' => terminal.clear_line_from_cursor(),
            'S' => {
                for _ in 0..first {
                    terminal.scroll_up();
                }
            }
            'T' => {
                for _ in 0..first {
                    let row = terminal.cells.remove(terminal.cells.len() - 1);
                    terminal.cells.insert(0, row);
                }
            }
            'm' => apply_sgr(&params, terminal),
            'h' | 'l' => {
                if self.csi.starts_with('?') {
                    let mode_str = &self.csi[1..];
                    if let Ok(mode) = mode_str.parse::<u32>() {
                        let enabled = command == 'h';
                        terminal.set_mode(mode, enabled);
                    }
                }
            }
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
            1 => terminal.graphics.intensity = Intensity::Bold,
            2 => terminal.graphics.intensity = Intensity::Dim,
            3 => terminal.graphics.italic = true,
            4 => terminal.graphics.underline = true,
            7 => terminal.graphics.reverse = true,
            9 => terminal.graphics.strikethrough = true,
            22 => terminal.graphics.intensity = Intensity::Normal,
            23 => terminal.graphics.italic = false,
            24 => terminal.graphics.underline = false,
            27 => terminal.graphics.reverse = false,
            29 => terminal.graphics.strikethrough = false,
            30..=37 | 90..=97 => terminal.set_ansi_fg(param),
            39 => terminal.graphics.fg = RgbColor::DEFAULT_FG,
            40..=47 | 100..=107 => terminal.set_ansi_bg(param),
            49 => terminal.graphics.bg = None,
            38 => {
                if iter.next() == Some(2) {
                    let (Some(r), Some(g), Some(b)) = (iter.next(), iter.next(), iter.next())
                    else {
                        continue;
                    };
                    terminal.graphics.fg = RgbColor {
                        r: (r.clamp(0, 255) as f32) / 255.0,
                        g: (g.clamp(0, 255) as f32) / 255.0,
                        b: (b.clamp(0, 255) as f32) / 255.0,
                    };
                }
            }
            48 => {
                if iter.next() == Some(2) {
                    let (Some(r), Some(g), Some(b)) = (iter.next(), iter.next(), iter.next())
                    else {
                        continue;
                    };
                    terminal.graphics.bg = Some(RgbColor {
                        r: (r.clamp(0, 255) as f32) / 255.0,
                        g: (g.clamp(0, 255) as f32) / 255.0,
                        b: (b.clamp(0, 255) as f32) / 255.0,
                    });
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

    #[test]
    fn parser_applies_background_color() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[44mB\x1b[49mN", &mut terminal);

        assert_eq!(terminal.cells()[0][0].ch, 'B');
        assert_eq!(
            terminal.cells()[0][0].bg,
            Some(RgbColor {
                r: 0.38,
                g: 0.62,
                b: 1.00
            })
        );
        assert_eq!(terminal.cells()[0][1].bg, None);
    }

    #[test]
    fn parser_applies_truecolor_background() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[48;2;10;20;30mX", &mut terminal);

        assert_eq!(
            terminal.cells()[0][0].bg,
            Some(RgbColor {
                r: 10.0 / 255.0,
                g: 20.0 / 255.0,
                b: 30.0 / 255.0
            })
        );
    }

    #[test]
    fn parser_applies_dim_intensity() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[2;31mD", &mut terminal);

        let fg = terminal.cells()[0][0].fg;
        assert!((fg.r - 0.55).abs() < 0.001);
        assert!((fg.g - 0.1265).abs() < 0.001);
        assert!((fg.b - 0.231).abs() < 0.001);
    }

    #[test]
    fn resize_preserves_existing_cells() {
        let mut terminal = TerminalBuffer::new(4, 2);

        for ch in "ab\ncd".chars() {
            terminal.put_char(ch);
        }
        terminal.resize(6, 3);

        assert_eq!(terminal.visible_lines()[0], "ab");
        assert_eq!(terminal.visible_lines()[1], "cd");
    }

    #[test]
    fn parser_applies_underline() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[4mU\x1b[24mN", &mut terminal);

        assert!(terminal.cells()[0][0].underline);
        assert!(!terminal.cells()[0][1].underline);
    }

    #[test]
    fn parser_applies_italic() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[3mI\x1b[23mN", &mut terminal);

        assert!(terminal.cells()[0][0].italic);
        assert!(!terminal.cells()[0][1].italic);
    }

    #[test]
    fn parser_applies_reverse_video() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[7mR\x1b[27mN", &mut terminal);

        assert!(terminal.cells()[0][0].reverse);
        assert!(!terminal.cells()[0][1].reverse);
    }

    #[test]
    fn parser_applies_strikethrough() {
        let mut terminal = TerminalBuffer::new(20, 3);
        let mut parser = OutputParser::default();

        parser.feed(b"\x1b[9mS\x1b[29mN", &mut terminal);

        assert!(terminal.cells()[0][0].strikethrough);
        assert!(!terminal.cells()[0][1].strikethrough);
    }

    #[test]
    fn scrollback_stores_scrolled_lines() {
        let mut terminal = TerminalBuffer::new(10, 2);

        for i in 0..5 {
            let line = format!("line{}\n", i);
            for ch in line.chars() {
                terminal.put_char(ch);
            }
        }

        assert!(terminal.scrollback.len() >= 3);
        assert_eq!(terminal.visible_lines()[0], "line4");
    }

    #[test]
    fn scroll_page_up_shows_older_content() {
        let mut terminal = TerminalBuffer::new(10, 2);

        for i in 0..5 {
            let line = format!("line{}\n", i);
            for ch in line.chars() {
                terminal.put_char(ch);
            }
        }

        terminal.scroll_page_up();
        let display = terminal.display_cells();
        assert_eq!(display[0][0].ch, 'l');
        assert_eq!(display[0][1].ch, 'i');
    }

    #[test]
    fn scroll_page_down_returns_toward_present() {
        let mut terminal = TerminalBuffer::new(10, 2);

        for i in 0..5 {
            let line = format!("line{}\n", i);
            for ch in line.chars() {
                terminal.put_char(ch);
            }
        }

        terminal.scroll_page_up();
        terminal.scroll_page_down();
        assert_eq!(terminal.scroll_offset, 0);
    }
}
