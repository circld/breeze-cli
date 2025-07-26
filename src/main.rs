mod cli;
mod core;
mod error;
mod fs;

use anyhow::Result;
use clap::Parser;
use cli::args::Args;
use core::explorer::Explorer;
use crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{
        Color, Modifier, Style, Stylize,
        palette::tailwind::{BLUE, SLATE},
    },
    symbols,
    text::Line,
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget,
        Widget,
    },
};
use std::io::{BufWriter, IsTerminal, Read, Stderr, stderr, stdin};
use std::path::PathBuf;
use std::{fmt, fs::DirEntry};

const HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;

fn main() -> Result<()> {
    let args = if stdin().is_terminal() {
        Args::parse()
    } else {
        let mut buffer = String::new();
        buffer.push_str("breeze-cli ");
        let _ = stdin().read_to_string(&mut buffer)?;
        Args::parse_from(buffer.trim().split_whitespace())
    };

    let explorer = Explorer::new(args.directory.canonicalize()?)?;
    let cwd = explorer.cwd();
    let paths = explorer.ls()?;
    let handle = stderr();

    let backend = CrosstermBackend::new(BufWriter::new(&handle));
    let terminal = Terminal::new(backend)?;
    let mut app = App {
        handle: &handle,
        should_exit: false,
        path_list: PathList::from_iter(paths),
        explorer: explorer,
        output: Output::new(cwd),
    };
    let result = app.run(terminal);
    println!("{}", app.output);
    result
}

struct App<'a> {
    handle: &'a Stderr,
    should_exit: bool,
    path_list: PathList,
    explorer: Explorer,
    output: Output,
}

struct Output {
    cwd: String,
    command: String,
    items: Vec<String>,
}

impl Output {
    fn new(cwd: String) -> Self {
        Output {
            cwd,
            command: "no-op".to_string(),
            items: Vec::new(),
        }
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.cwd, self.command, self.items.join(" "))
    }
}

struct PathList {
    items: Vec<Path>,
    state: ListState,
}

struct Path {
    value: String,
    kind: ObjectType,
}

impl Path {
    fn new(value: String, kind: ObjectType) -> Self {
        Self { value, kind }
    }
}

enum ObjectType {
    File,
    Directory,
}

impl From<PathBuf> for ObjectType {
    fn from(path_buf: PathBuf) -> Self {
        match path_buf.is_dir() {
            true => ObjectType::Directory,
            false => ObjectType::File,
        }
    }
}

impl FromIterator<PathBuf> for PathList {
    fn from_iter<I: IntoIterator<Item = PathBuf>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|pb| Path::new(pb.to_string_lossy().to_string(), ObjectType::from(pb)))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

impl FromIterator<DirEntry> for PathList {
    fn from_iter<I: IntoIterator<Item = DirEntry>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|de| {
                Path::new(
                    de.file_name().to_string_lossy().to_string(),
                    ObjectType::from(de.path()),
                )
            })
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

impl App<'_> {
    fn run(&mut self, mut terminal: Terminal<CrosstermBackend<BufWriter<&Stderr>>>) -> Result<()> {
        enable_raw_mode()?;
        self.handle.execute(EnterAlternateScreen)?;
        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut *self, frame.area()))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            };
        }

        self.handle.execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_exit = true,
            KeyCode::Esc => self.select_none(),
            KeyCode::Char('j') | KeyCode::Down => self.select_next(),
            KeyCode::Char('k') | KeyCode::Up => self.select_previous(),
            KeyCode::Char('g') | KeyCode::Home => self.select_first(),
            KeyCode::Char('G') | KeyCode::End => self.select_last(),
            KeyCode::Char('l') | KeyCode::Right => self.enter_directory(),
            KeyCode::Char('h') | KeyCode::Left => self.change_to_parent(),
            KeyCode::Enter => self.update_command("do-thing".to_string(), true),
            _ => {}
        }
    }

    fn select_none(&mut self) {
        self.path_list.state.select(None);
    }

    fn select_next(&mut self) {
        self.path_list.state.select_next();
    }
    fn select_previous(&mut self) {
        self.path_list.state.select_previous();
    }

    fn select_first(&mut self) {
        self.path_list.state.select_first();
    }

    fn select_last(&mut self) {
        self.path_list.state.select_last();
    }

    // TODO better error handling
    // TODO more ergonomic path_list construction from Vec<String> (helper func?)
    fn enter_directory(&mut self) {
        if let Some(i) = self.path_list.state.selected() {
            if let ObjectType::Directory = self.path_list.items[i].kind {
                let full_path = self
                    .explorer
                    .current_dir
                    .join(self.path_list.items[i].value.to_string());
                let new_paths = self
                    .explorer
                    .cd(full_path.into())
                    .expect("Could not enter directory!");
                self.path_list = PathList::from_iter(new_paths)
            }
        }
    }

    fn change_to_parent(&mut self) {
        let current = &self.explorer.current_dir;
        let parent = self
            .explorer
            .current_dir
            .parent()
            .unwrap_or(current.as_path())
            .to_path_buf();
        let new_paths = self
            .explorer
            .cd(parent)
            .expect("change to parent failed on cd");
        self.path_list = PathList::from_iter(new_paths)
    }

    fn update_command(&mut self, command: String, quit: bool) {
        self.output.command = command;
        if let Some(i) = self.path_list.state.selected() {
            let cwd = self.explorer.cwd();
            self.output.items = vec![self.path_list.items[i].value.to_string()]
                .iter()
                .map(|s| format!("{}/{}", cwd, s))
                .collect();
        }
        if quit {
            self.should_exit = true;
        }
    }
}

impl Widget for &mut App<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);
        self.render_list(main_area, buf);
    }
}

impl App<'_> {
    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("MVP Breeze TUI")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Use ↓↑ to move, ← to unselect, → to change status, g/G to go top/bottom.")
            .centered()
            .render(area, buf);
    }

    // TODO move these explorer implementation details to src/core/explorer.rs
    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw(self.explorer.cwd()))
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self
            .path_list
            .items
            .iter()
            .enumerate()
            .map(|(_, path_item)| ListItem::from(path_item).bg(NORMAL_ROW_BG))
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        StatefulWidget::render(list, area, buf, &mut self.path_list.state);
    }
}

impl From<&Path> for ListItem<'_> {
    fn from(path: &Path) -> Self {
        let line = Line::styled(format!("{}", path.value), TEXT_FG_COLOR);
        ListItem::new(line)
    }
}
