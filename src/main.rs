#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::else_if_without_else)]
mod document;
mod editor;
mod filetype;
mod highlighting;
mod row;
mod terminal;

use document::Document;
use editor::Editor;
use editor::Position;
use editor::SearchDirection;
use filetype::FileType;
use filetype::HighlightingOptions;
use row::Row;
use terminal::Terminal;

fn main() {
    Editor::default().run();
}
