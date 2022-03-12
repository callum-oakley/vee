use {
    crate::line::Line,
    anyhow::Result,
    crossterm::event::{KeyCode, KeyEvent},
    regex::Regex,
    std::{fmt, fs, result},
    unicode_width::{UnicodeWidthChar, UnicodeWidthStr},
};

// A comment with some 中文 to test proper unicode handling.
// This line has fewer chars, but is the same visual length.

// field order in Cursor and Point is important for the PartialOrd derivation
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Cursor {
    pub y: usize, // row
    pub x: usize, // actual col (in bytes)
    w: usize,     // target col (in visual width)
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Point {
    pub y: usize,
    pub x: usize,
}

impl From<Cursor> for Point {
    fn from(cursor: Cursor) -> Self {
        Point {
            x: cursor.x,
            y: cursor.y,
        }
    }
}

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    System,
    // Search,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert => write!(f, "INSERT"),
            Mode::System => write!(f, "SYSTEM"),
            // Mode::Search => write!(f, "SEARCH"),
        }
    }
}

pub struct State {
    pub mode: Mode,
    pub file: String,
    pub text: Vec<Line>,
    pub cursor: Cursor,
    pub anchor: Option<Cursor>,
    pub search: Option<result::Result<Regex, regex::Error>>,
}

impl State {
    pub fn new(file: String) -> Result<Self> {
        let text = fs::read_to_string(&file)?
            .lines()
            .map(|s| Line::new(s.to_string(), None))
            .collect();
        Ok(Self {
            mode: Mode::Normal,
            file,
            text,
            cursor: Cursor { w: 0, x: 0, y: 0 },
            anchor: None,
            search: None,
        })
    }

    pub fn handle(&mut self, event: KeyEvent) -> bool {
        match self.mode {
            Mode::Normal => {
                match event.code {
                    KeyCode::Char('q') => self.select_inside_quotes(),
                    KeyCode::Char('w') => self.select_word(|c| c.is_alphanumeric() || c == '_'),
                    KeyCode::Char('e') => self.select_inside_brackets(),
                    KeyCode::Char('r') => self.select_line(),
                    KeyCode::Char('y') => self.move_start_of_line(),
                    KeyCode::Char('u') => self.move_left_word(|c| c.is_alphanumeric() || c == '_'),
                    KeyCode::Char('i') => self.move_right_word(|c| c.is_alphanumeric() || c == '_'),
                    KeyCode::Char('o') => self.move_end_of_line(),
                    KeyCode::Char('p') => self.move_bracket_inside(),
                    KeyCode::Char('s') => self.anchor = Some(self.cursor),
                    KeyCode::Char('f') => self.begin_edit(),
                    KeyCode::Char('h') | KeyCode::Left => self.move_left(1),
                    KeyCode::Char('j') | KeyCode::Down => self.move_down(1),
                    KeyCode::Char('k') | KeyCode::Up => self.move_up(1),
                    KeyCode::Char('l') | KeyCode::Right => self.move_right(1),
                    KeyCode::Char('n') => self.move_start_of_file(),
                    KeyCode::Char('m') => self.move_next_match(),
                    KeyCode::Char(',') => self.move_prev_match(),
                    KeyCode::Char('.') => self.move_end_of_file(),
                    KeyCode::Char('/') => self.search(),
                    KeyCode::Char('Q') => self.select_outside_quotes(),
                    KeyCode::Char('W') => self.select_word(|c| !c.is_whitespace()),
                    KeyCode::Char('E') => self.select_outside_brackets(),
                    KeyCode::Char('R') => self.select_para(),
                    KeyCode::Char('Y') => self.move_start_of_para(),
                    KeyCode::Char('U') => self.move_left_word(|c| !c.is_whitespace()),
                    KeyCode::Char('I') => self.move_right_word(|c| !c.is_whitespace()),
                    KeyCode::Char('O') => self.move_end_of_para(),
                    KeyCode::Char('P') => self.move_bracket_outside(),
                    KeyCode::Char('H') => self.move_left(5),
                    KeyCode::Char('J') => self.move_down(5),
                    KeyCode::Char('K') => self.move_up(5),
                    KeyCode::Char('L') => self.move_right(5),
                    KeyCode::Esc => {
                        if self.anchor.is_some() {
                            self.anchor = None
                        } else {
                            self.cancel_search()
                        }
                    }
                    KeyCode::Char(' ') => {
                        self.mode = Mode::System;
                    }
                    _ => (),
                };
            }
            Mode::Insert => match event.code {
                KeyCode::Esc => self.end_edit(),
                _ => (),
            },
            Mode::System => match event.code {
                KeyCode::Char('q') => {
                    return false;
                }
                _ => {
                    self.mode = Mode::Normal;
                }
            },
        }
        true
    }

    pub fn cursor_width(&self) -> usize {
        self.text[self.cursor.y].0[..self.cursor.x].width()
    }

    pub fn selection(&self) -> Option<(Cursor, Cursor)> {
        self.anchor.map(|anchor| {
            if anchor < self.cursor {
                (anchor, self.cursor)
            } else {
                (self.cursor, anchor)
            }
        })
    }

    fn search(&mut self) {
        if let Some(selection) = self.selection() {
            if selection.0.y == selection.1.y {
                self.search = Some(Regex::new(&regex::escape(
                    &self.text[selection.0.y].0[selection.0.x..selection.1.x],
                )));
                for line in &mut self.text {
                    line.annotate(self.search.as_ref().and_then(|r| r.as_ref().ok()));
                }
            }
        }
    }

    fn cancel_search(&mut self) {
        self.search = None;
        for line in &mut self.text {
            line.annotate(None);
        }
    }

    fn move_cursor(&mut self, point: Point) {
        self.cursor.y = point.y;
        self.cursor.x = point.x;
        self.cursor.w = self.cursor_width();
    }

    fn prev_char(&self, point: Point) -> Option<char> {
        self.text[point.y].0[..point.x].chars().last()
    }

    fn next_char(&self, point: Point) -> Option<char> {
        self.text[point.y].0[point.x..].chars().next()
    }

    fn left_of(&self, point: Point) -> Option<Point> {
        self.prev_char(point).map(|c| Point {
            x: point.x - c.len_utf8(),
            ..point
        })
    }

    fn right_of(&self, point: Point) -> Option<Point> {
        self.next_char(point).map(|c| Point {
            x: point.x + c.len_utf8(),
            ..point
        })
    }

    fn update_x(&mut self) {
        let mut w = 0;
        self.cursor.x = 0;
        for (x, c) in self.text[self.cursor.y].0.char_indices() {
            self.cursor.x = x;
            w += c.width().unwrap_or(0);
            if w > self.cursor.w {
                return;
            }
        }
        self.cursor.x = self.text[self.cursor.y].0.len()
    }

    fn move_up(&mut self, dist: usize) {
        if self.cursor.y > dist {
            self.cursor.y -= dist;
        } else {
            self.cursor.y = 0;
        }
        self.update_x();
    }

    fn move_down(&mut self, dist: usize) {
        if self.cursor.y + dist < self.text.len() {
            self.cursor.y += dist;
        } else {
            self.cursor.y = self.text.len() - 1
        }
        self.update_x();
    }

    fn move_left(&mut self, dist: usize) {
        for _ in 0..dist {
            if let Some(c) = self.prev_char(self.cursor.into()) {
                self.cursor.x -= c.len_utf8();
            }
        }
        self.cursor.w = self.cursor_width();
    }

    fn move_right(&mut self, dist: usize) {
        for _ in 0..dist {
            if let Some(c) = self.next_char(self.cursor.into()) {
                self.cursor.x += c.len_utf8();
            }
        }
        self.cursor.w = self.cursor_width();
    }

    fn left_word(&self, mut wordish: impl FnMut(char) -> bool, point: Point) -> Option<Point> {
        let mut point = point;
        let mut seen_word = self.next_char(point).map_or(false, &mut wordish);
        for c in self.text[point.y].0[..point.x].chars().rev() {
            if seen_word && !wordish(c) {
                break;
            } else if !seen_word && wordish(c) {
                seen_word = true;
            }
            point.x -= c.len_utf8();
        }
        if seen_word {
            Some(point)
        } else {
            None
        }
    }

    fn right_word(&self, mut wordish: impl FnMut(char) -> bool, point: Point) -> Option<Point> {
        let mut point = point;
        let mut seen_word = self.prev_char(point).map_or(false, &mut wordish);
        for c in self.text[point.y].0[point.x..].chars() {
            if seen_word && !wordish(c) {
                break;
            } else if !seen_word && wordish(c) {
                seen_word = true;
            }
            point.x += c.len_utf8();
        }
        if seen_word {
            Some(point)
        } else {
            None
        }
    }

    fn move_left_word(&mut self, wordish: impl FnMut(char) -> bool) {
        if let Some(left) = self.left_of(self.cursor.into()) {
            if let Some(point) = self.left_word(wordish, left) {
                self.move_cursor(point);
            }
        }
    }

    fn move_right_word(&mut self, wordish: impl FnMut(char) -> bool) {
        if let Some(right) = self.right_of(self.cursor.into()) {
            if let Some(point) = self.right_word(wordish, right) {
                self.move_cursor(point);
            }
        }
    }

    fn start_of_line(&self, y: usize) -> Point {
        for (x, c) in self.text[y].0.char_indices() {
            if !c.is_whitespace() {
                return Point { x, y };
            }
        }
        Point { x: 0, y }
    }

    fn end_of_line(&self, y: usize) -> Point {
        Point {
            x: self.text[y].0.len(),
            y,
        }
    }

    fn move_start_of_line(&mut self) {
        self.move_cursor(self.start_of_line(self.cursor.y));
    }

    fn move_end_of_line(&mut self) {
        self.move_cursor(self.end_of_line(self.cursor.y));
    }

    fn open_quote(&self, point: Point) -> Option<Point> {
        for y in (0..=point.y).rev() {
            let mut x = if y == point.y {
                point.x
            } else {
                self.text[y].0.len()
            };
            for c in self.text[y].0[..x].chars().rev() {
                x -= c.len_utf8();
                if c == '"' {
                    return Some(Point { x, y });
                }
            }
        }
        None
    }

    fn close_quote(&self, point: Point) -> Option<Point> {
        for y in point.y..self.text.len() {
            let mut x = if y == point.y { point.x } else { 0 };
            for c in self.text[y].0[x..].chars() {
                if c == '"' {
                    return Some(Point { x, y });
                }
                x += c.len_utf8();
            }
        }
        None
    }

    fn close_bracket(&self, point: Point) -> Option<Point> {
        let mut pending = Vec::new();
        for y in point.y..self.text.len() {
            let mut x = if y == point.y { point.x } else { 0 };
            for c in self.text[y].0[x..].chars() {
                match (c, pending.last()) {
                    ('[' | '{' | '(', _) => pending.push(c),
                    (']', Some('[')) | ('}', Some('{')) | (')', Some('(')) => {
                        pending.pop();
                    }
                    (']' | '}' | ')', _) => {
                        return Some(Point { x, y });
                    }
                    _ => (),
                }
                x += c.len_utf8();
            }
        }
        None
    }

    fn open_bracket(&self, point: Point) -> Option<Point> {
        let mut pending = Vec::new();
        for y in (0..=point.y).rev() {
            let mut x = if y == point.y {
                point.x
            } else {
                self.text[y].0.len()
            };
            for c in self.text[y].0[..x].chars().rev() {
                x -= c.len_utf8();
                match (c, pending.last()) {
                    (']' | '}' | ')', _) => pending.push(c),
                    ('[', Some(']')) | ('{', Some('}')) | ('(', Some(')')) => {
                        pending.pop();
                    }
                    ('[' | '{' | '(', _) => {
                        return Some(Point { x, y });
                    }
                    _ => (),
                }
            }
        }
        None
    }

    fn start_of_para(&self, point: Point) -> Point {
        let mut point = point;
        while point.y > 1 {
            if !self.text[point.y].0.is_empty() && self.text[point.y - 1].0.is_empty() {
                return self.start_of_line(point.y);
            }
            point.y -= 1;
        }
        self.start_of_file()
    }

    fn end_of_para(&self, point: Point) -> Point {
        let mut point = point;
        while point.y + 1 < self.text.len() {
            if !self.text[point.y].0.is_empty() && self.text[point.y + 1].0.is_empty() {
                return self.end_of_line(point.y);
            }
            point.y += 1;
        }
        self.end_of_file()
    }

    fn move_bracket_inside(&mut self) {
        if let Some(']' | '}' | ')') = self.next_char(self.cursor.into()) {
            if let Some(Point { x, y }) = self.open_bracket(self.cursor.into()) {
                self.move_cursor(Point { y, x: x + 1 });
            }
        } else if let Some(Point { x, y }) = self.close_bracket(self.cursor.into()) {
            self.move_cursor(Point { y, x });
        }
    }

    fn move_bracket_outside(&mut self) {
        if let Some('[' | '{' | '(') = self.next_char(self.cursor.into()) {
            if let Some(Point { x, y }) = self.close_bracket(Point {
                x: self.cursor.x + 1,
                y: self.cursor.y,
            }) {
                self.move_cursor(Point { y, x: x + 1 });
            }
        } else if let Some(']' | '}' | ')') = self.prev_char(self.cursor.into()) {
            if let Some(Point { x, y }) = self.open_bracket(Point {
                x: self.cursor.x - 1,
                y: self.cursor.y,
            }) {
                self.move_cursor(Point { y, x });
            }
        }
    }

    fn move_start_of_para(&mut self) {
        self.move_up(1);
        self.move_cursor(self.start_of_para(self.cursor.into()));
    }

    fn move_end_of_para(&mut self) {
        self.move_down(1);
        self.move_cursor(self.end_of_para(self.cursor.into()));
    }

    fn start_of_file(&self) -> Point {
        Point { x: 0, y: 0 }
    }

    fn end_of_file(&self) -> Point {
        self.end_of_line(self.text.len() - 1)
    }

    fn move_start_of_file(&mut self) {
        self.move_cursor(self.start_of_file());
    }

    fn move_end_of_file(&mut self) {
        self.move_cursor(self.end_of_file());
    }

    fn begin_edit(&mut self) {
        self.mode = Mode::Insert;
    }

    fn end_edit(&mut self) {
        self.mode = Mode::Normal;
    }

    fn select_word(&mut self, mut wordish: impl FnMut(char) -> bool) {
        if let Some(left) = self.left_word(&mut wordish, self.cursor.into()) {
            if let Some(right) = self.right_word(&mut wordish, self.cursor.into()) {
                self.move_cursor(left);
                self.anchor = Some(self.cursor);
                self.move_cursor(right);
            }
        }
    }

    fn select_inside_brackets(&mut self) {
        if let Some(open) = self.open_bracket(self.cursor.into()) {
            if let Some(close) = self.close_bracket(self.cursor.into()) {
                self.move_cursor(Point {
                    x: open.x + 1,
                    ..open
                });
                self.anchor = Some(self.cursor);
                self.move_cursor(close);
            }
        }
    }

    fn select_outside_brackets(&mut self) {
        if let Some('[' | '{' | '(') = self.next_char(self.cursor.into()) {
            self.move_right(1);
        } else if let Some(']' | '}' | ')') = self.prev_char(self.cursor.into()) {
            self.move_left(1);
        }
        self.select_inside_brackets();
        self.grow_selection();
    }

    fn select_inside_quotes(&mut self) {
        if let Some(open) = self.open_quote(self.cursor.into()) {
            if let Some(close) = self.close_quote(self.cursor.into()) {
                self.move_cursor(Point {
                    x: open.x + 1,
                    ..open
                });
                self.anchor = Some(self.cursor);
                self.move_cursor(close);
            }
        }
    }

    fn select_outside_quotes(&mut self) {
        if let Some('"') = self.prev_char(self.cursor.into()) {
            self.move_left(1);
        }
        self.select_inside_quotes();
        self.grow_selection();
    }

    fn select_line(&mut self) {
        self.move_start_of_line();
        self.anchor = Some(self.cursor);
        self.move_end_of_line();
    }

    fn select_para(&mut self) {
        self.move_cursor(self.start_of_para(self.cursor.into()));
        self.anchor = Some(self.cursor);
        self.move_cursor(self.end_of_para(self.cursor.into()));
    }

    // assumes anchor is before cursor
    fn grow_selection(&mut self) {
        self.invert_selection();
        self.move_left(1);
        self.invert_selection();
        self.move_right(1);
    }

    fn invert_selection(&mut self) {
        if let Some(anchor) = self.anchor {
            self.anchor = Some(self.cursor);
            self.cursor = anchor;
        }
    }
}
