#![warn(clippy::all, clippy::pedantic)] // super long comment to test horizontal scrolling super long comment to test horizontal scrolling super long comment to test horizontal scrolling
use crate::Document;
use crate::Row;
use crate::Terminal;

use std::env;
use std::time::Duration;
use std::time::Instant;
use termion::color;
use termion::event::Key;

const STATUS_FG_COLOR: color::Rgb = color::Rgb(63, 63, 63);
const STATUS_BG_COLOR: color::Rgb = color::Rgb(239, 239, 239);
const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 1;

#[derive(PartialEq, Copy, Clone)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Default, Clone)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

struct StatusMessage {
    text: String,
    time: Instant,
}

impl StatusMessage {
    fn from(message: String) -> Self {
        Self {
            text: message,
            time: Instant::now(),
        }
    }
}

pub struct Editor {
    should_quit: bool,
    terminal: Terminal,
    cursor_position: Position,
    offset: Position,
    document: Document,
    status_message: StatusMessage,
    quit_times: u8,
    highlighted_word: Option<String>,
}

impl Editor {
    pub fn default() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut initial_status = String::from("HELP: Alt-F = find | Alt-S = save | Alt-Q = quit");

        let document = if let Some(filename) = args.get(1) {
            let doc = Document::open(filename);

            if let Ok(doc) = doc {
                doc
            } else {
                initial_status = format!("ERR: Could not open file: {filename}");
                Document::default()
            }
        } else {
            Document::default()
        };

        Editor {
            should_quit: false,
            terminal: Terminal::init().expect("Failed to initialize terminal"),
            cursor_position: Position::default(),
            offset: Position::default(),
            document,
            status_message: StatusMessage::from(initial_status),
            quit_times: QUIT_TIMES,
            highlighted_word: None,
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Err(error) = self.refresh_screen() {
                die(&error);
            }

            if self.should_quit {
                break;
            }

            if let Err(error) = self.process_keypress() {
                die(&error);
            }
        }
    }

    fn refresh_screen(&mut self) -> Result<(), std::io::Error> {
        Terminal::cursor_hide();
        Terminal::cursor_position(&Position::default());

        if self.should_quit {
            Terminal::clear_screen();
            println!("Goodbye.\r");
        } else {
            self.document.highlight(
                &self.highlighted_word,
                Some(self.offset.y.saturating_add(self.terminal.size().height as usize))
            );
            self.draw_rows();
            self.draw_status_bar();
            self.draw_message_bar();
            Terminal::cursor_position(&Position {
                x: self.cursor_position.x.saturating_sub(self.offset.x),
                y: self.cursor_position.y.saturating_sub(self.offset.y),
            });
        }

        Terminal::cursor_show();
        Terminal::flush()
    }

    pub fn draw_row(&self, row: &Row) {
        let start = self.offset.x;
        let width = self.terminal.size().width as usize;
        let end = start + width;

        let row = row.render(start, end);
        println!("{row}\r");
    }

    fn draw_rows(&self) {
        let height = self.terminal.size().height;

        for terminal_row in 0..height {
            Terminal::clear_current_line();

            if let Some(row) = self.document.row(terminal_row as usize + self.offset.y) {
                self.draw_row(row);
            } else if self.document.is_empty() && terminal_row == height / 3 {
                self.draw_welcome_message();
            } else {
                println!("~\r");
            }
        }
    }

    fn draw_status_bar(&self) {
        let width = self.terminal.size().width as usize;
        let modified_indicator = if self.document.is_dirty() {
            " (modified)"
        } else {
            ""
        };

        let mut filename = "[No Name]".to_string();
        if let Some(name) = &self.document.filename {
            filename = name.clone();
            filename.truncate(20);
        }
        let mut status = format!(
            "{} - {} lines{}",
            filename,
            self.document.len(),
            modified_indicator
        );

        let line_indicator = format!(
            "{} | {}/{}",
            self.document.file_type(),
            self.cursor_position.y.saturating_add(1),
            self.document.len()
        );
        let len = status.len() + line_indicator.len();

        status.push_str(&" ".repeat(width.saturating_sub(len)));

        status = format!("{status}{line_indicator}");

        status.truncate(width);
        Terminal::set_bg_color(STATUS_BG_COLOR);
        Terminal::set_fg_color(STATUS_FG_COLOR);
        println!("{status}\r");
        Terminal::reset_fg_color();
        Terminal::reset_bg_color();
    }

    fn draw_message_bar(&self) {
        Terminal::clear_current_line();
        let message = &self.status_message;
        if message.time.elapsed() < Duration::new(5, 0) {
            let mut text = message.text.clone();
            text.truncate(self.terminal.size().width as usize);
            print!("{text}");
        }
    }

    fn draw_welcome_message(&self) {
        let mut welcome_message = format!("Hecto editor -- version {VERSION}\r");
        let width = self.terminal.size().width as usize;
        let len = welcome_message.len();
        let padding = width.saturating_sub(len) / 2;
        let spaces = " ".repeat(padding.saturating_sub(1));

        welcome_message = format!("~{spaces}{welcome_message}");
        welcome_message.truncate(width);

        println!("{welcome_message}\r");
    }

    fn save(&mut self) {
        if self.document.filename.is_none() {
            let new_name = self.prompt("Save as: ", |_, _, _| {}).unwrap_or(None);

            if new_name.is_none() {
                self.status_message = StatusMessage::from("Save aborted.".to_string());
                return;
            }

            self.document.filename = new_name;
        }

        if self.document.save().is_ok() {
            self.status_message = StatusMessage::from("File saved successfully".to_string());
        } else {
            self.status_message = StatusMessage::from("Error writing file".to_string());
        }
    }

    fn search(&mut self) {
        let old_position = self.cursor_position.clone();
        let mut direction = SearchDirection::Forward;

        let query = self.prompt("Search: (ESC to cancel, Arrows to navigate): ", |editor, key, query| {
            let mut moved = false;
            match key {
                Key::Right | Key::Down => {
                    direction = SearchDirection::Forward;
                    editor.move_cursor(Key::Right);
                    moved = true;
                }
                Key::Left | Key::Up => direction = SearchDirection::Backward,
                _ => direction = SearchDirection::Forward,
            }
            if let Some(position) = editor.document.find(query, &editor.cursor_position, direction) {
                editor.cursor_position = position;
                editor.scroll();
            } else if moved {
                editor.move_cursor(Key::Left);
            }

            editor.highlighted_word = Some(query.to_string());
        })
        .unwrap_or(None);
        
        if query.is_none() {
            self.cursor_position = old_position;
            self.scroll();
        }

        self.highlighted_word = None;
    }

    fn process_keypress(&mut self) -> Result<(), std::io::Error> {
        let pressed_key = Terminal::read_key()?;

        match pressed_key {
            Key::Alt('q') => {
                if self.quit_times > 0 && self.document.is_dirty() {
                    self.status_message = StatusMessage::from("WARNING! File has unsaved changes. Press Alt-Q again to quit".to_string());
                    self.quit_times -= 1;
                    return Ok(());
                }

                self.should_quit = true;
            }
            Key::Alt('s') => self.save(),
            Key::Alt('f') => self.search(),
            Key::Char(c) => {
                self.document.insert(&self.cursor_position, c);
                self.move_cursor(Key::Right);
            }
            Key::Delete => self.document.delete(&self.cursor_position),
            Key::Backspace => {
                if self.cursor_position.x > 0 || self.cursor_position.y > 0 {
                    self.move_cursor(Key::Left);
                    self.document.delete(&self.cursor_position);
                }
            }
            Key::Up
            | Key::Down
            | Key::Left
            | Key::Right
            | Key::PageUp
            | Key::PageDown
            | Key::Home
            | Key::End => self.move_cursor(pressed_key),
            _ => (),
        }

        self.scroll();

        if self.quit_times < QUIT_TIMES {
            self.quit_times = QUIT_TIMES;
            self.status_message = StatusMessage::from(String::new());
        }

        Ok(())
    }

    fn prompt<C>(&mut self, prompt: &str, mut callback: C) -> Result<Option<String>, std::io::Error> where C: FnMut(&mut Self, Key, &String), {
        let mut result = String::new();

        loop {
            self.status_message = StatusMessage::from(format!("{prompt}{result}"));
            self.refresh_screen()?;

            let key = Terminal::read_key()?;
            match key {
                Key::Backspace => result.truncate(result.len().saturating_sub(1)),
                Key::Char('\n') => break,
                Key::Char(c) => {
                    if !c.is_control() {
                        result.push(c);
                    }
                }
                Key::Esc => {
                    result.truncate(0);
                    break;
                }
                _ => (),
            }

            callback(self, key, &result);
        }

        self.status_message = StatusMessage::from(String::new());
        if result.is_empty() {
            return Ok(None);
        }

        Ok(Some(result))
    }

    fn scroll(&mut self) {
        let Position { x, y } = self.cursor_position;
        let width = self.terminal.size().width as usize;
        let height = self.terminal.size().height as usize;
        let offset = &mut self.offset;

        if y < offset.y {
            // scroll up
            offset.y = y;
        } else if y >= offset.y.saturating_add(height) {
            // scroll down
            offset.y = y.saturating_sub(height).saturating_add(1);
        }

        if x < offset.x {
            // scroll left
            offset.x = x;
        } else if x >= offset.x.saturating_add(width) {
            // scroll right
            offset.x = x.saturating_sub(width).saturating_add(1);
        }
    }

    fn move_cursor(&mut self, key: Key) {
        let terminal_height = self.terminal.size().height as usize;
        let Position { mut x, mut y } = self.cursor_position;

        let height = self.document.len();
        let mut width = if let Some(row) = self.document.row(y) {
            row.len()
        } else {
            0
        };

        match key {
            Key::Up => y = y.saturating_sub(1),
            Key::Down => {
                if y < height {
                    y = y.saturating_add(1);
                }
            }
            Key::Left => {
                if x > 0 {
                    x -= 1;
                } else if y > 0 {
                    y -= 1;

                    if let Some(row) = self.document.row(y) {
                        x = row.len();
                    } else {
                        x = 0;
                    }
                }
            }
            Key::Right => {
                if x < width {
                    x += 1;
                } else if y < height {
                    y += 1;
                    x = 0;
                }
            }
            Key::PageUp => {
                y = if y > terminal_height {
                    y - terminal_height
                } else {
                    0
                }
            }
            Key::PageDown => {
                y = if y.saturating_add(terminal_height) < height {
                    y + terminal_height
                } else {
                    height
                }
            }
            Key::Home => x = 0,
            Key::End => x = width,
            _ => (),
        }

        width = if let Some(row) = self.document.row(y) {
            // width needs to be recalculated for the new row
            row.len()
        } else {
            0
        };

        if x > width {
            x = width;
        }

        self.cursor_position = Position { x, y };
    }
}

fn die(e: &std::io::Error) {
    println!("{}", termion::clear::All);
    panic!("{}", e);
}
