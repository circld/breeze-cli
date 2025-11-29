mod cli;
mod core;
mod error;
mod fs;

use anyhow::{Context, Result};
use clap::Parser;
use cli::args::Args;
use core::explorer::Explorer;
use crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use error::ExplorerError;
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
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
use std::path::PathBuf;
use std::{
    io::{BufWriter, IsTerminal, Read, Stderr, stderr, stdin},
};
use std::{fmt, fs::DirEntry};

const HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800);
const TEXT_FG_COLOR: Color = SLATE.c200;

fn main() -> Result<(), ExplorerError> {
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
        matcher: Matcher::new(Config::DEFAULT.match_paths()),
        pattern: None,
        filter_string: String::new(),
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
    matcher: Matcher,
    pattern: Option<Pattern>,
    filter_string: String,
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
    match_indices: Vec<u32>,
}

impl Path {
    fn new(value: String, kind: ObjectType) -> Self {
        Self {
            value,
            kind,
            match_indices: Vec::new(),
        }
    }

    fn with_match_indices(value: String, kind: ObjectType, match_indices: Vec<u32>) -> Self {
        Self {
            value,
            kind,
            match_indices,
        }
    }
}

#[derive(Clone, Copy)]
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
    fn run(
        &mut self,
        mut terminal: Terminal<CrosstermBackend<BufWriter<&Stderr>>>,
    ) -> Result<(), ExplorerError> {
        let mut unhandled = Vec::new();

        enable_raw_mode()?;
        self.handle.execute(EnterAlternateScreen)?;
        while !self.should_exit {
            terminal.draw(|frame| frame.render_widget(&mut *self, frame.area()))?;
            if let Event::Key(key) = event::read()? {
                match self.handle_key(key) {
                    Ok(_) => (),
                    err => {
                        let i = self.path_list.state.selected();
                        let selected = i
                            .map(|idx| self.path_list.items[idx].value.to_string())
                            .unwrap_or("nothing".to_string());
                        let msg = format!(
                            "Failed on key {:?} with {:?} selected",
                            key.code.to_string(),
                            selected
                        );
                        unhandled.push(err.context(msg))
                    }
                }
            };
        }

        self.handle.execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;

        if unhandled.len() > 0 {
            println!("{:?}", unhandled);
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<(), ExplorerError> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }
        match key.code {
            KeyCode::Char('Q') => self.should_exit = true,
            KeyCode::Esc => self.clear_filter(),
            KeyCode::Down => self.select_next(),
            KeyCode::Up => self.select_previous(),
            KeyCode::Home => self.select_first(),
            KeyCode::End => self.select_last(),
            KeyCode::Right => {
                self.enter_directory()?;
                self.clear_filter();
            }
            KeyCode::Left => {
                self.change_to_parent()?;
                self.clear_filter();
            }
            KeyCode::Enter => self.update_command("do-thing".to_string(), true),
            KeyCode::Char(c) => self.filter_paths(c),
            KeyCode::Backspace => self.remove_last_char_from_filter(),
            _ => (),
        }
        Ok(())
    }

    fn clear_filter(&mut self) {
        self.filter_string.clear();
        self.pattern = None;
        if let Ok(new_paths) = self.explorer.ls() {
            self.path_list = PathList::from_iter(new_paths);
        }
        // Auto-select first item after clearing filter
        self.path_list.state.select_first();
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

    fn enter_directory(&mut self) -> Result<(), ExplorerError> {
        if let Some(i) = self.path_list.state.selected() {
            if let ObjectType::Directory = self.path_list.items[i].kind {
                let full_path = self
                    .explorer
                    .current_dir
                    .join(self.path_list.items[i].value.to_string());
                let new_paths = self.explorer.cd(full_path.into())?;
                self.path_list = PathList::from_iter(new_paths);
            }
        }
        Ok(())
    }

    fn change_to_parent(&mut self) -> Result<(), ExplorerError> {
        let current = &self.explorer.current_dir;
        let parent = self
            .explorer
            .current_dir
            .parent()
            .unwrap_or(current.as_path())
            .to_path_buf();
        let new_paths = self.explorer.cd(parent)?;
        self.path_list = PathList::from_iter(new_paths);
        Ok(())
    }

    fn filter_paths(&mut self, c: char) {
        // Append new character to filter string
        self.filter_string.push(c);

        // Rebuild pattern from complete filter string
        let pattern = Pattern::parse(
            &self.filter_string,
            CaseMatching::Ignore,
            Normalization::Smart,
        );

        // Get all current items with their types
        let items_with_types: Vec<(String, ObjectType)> = self
            .path_list
            .items
            .iter()
            .map(|e| (e.value.to_string(), e.kind))
            .collect();

        // Match and collect indices for each item
        let mut new_items = Vec::new();
        for (value, kind) in items_with_types {
            let mut indices = Vec::new();
            let mut buf = Vec::new();
            let haystack = Utf32Str::new(&value, &mut buf);
            let score = pattern.indices(haystack, &mut self.matcher, &mut indices);
            if score.is_some() {
                new_items.push((value, kind, indices, score.unwrap()));
            }
        }

        // Sort by score (higher is better)
        new_items.sort_by(|a, b| b.3.cmp(&a.3));

        // Update path list with match indices
        self.path_list.items = new_items
            .into_iter()
            .map(|(value, kind, indices, _)| Path::with_match_indices(value, kind, indices))
            .collect();

        self.pattern = Some(pattern);

        // Auto-select first item in filtered list
        self.path_list.state.select_first();
    }

    fn remove_last_char_from_filter(&mut self) {
        // Remove last character from filter string
        self.filter_string.pop();

        if self.filter_string.is_empty() {
            // No filter - restore full directory listing
            if let Ok(new_paths) = self.explorer.ls() {
                self.path_list = PathList::from_iter(new_paths);
            }
            self.pattern = None;
        } else {
            // Rebuild pattern from updated filter string
            let pattern = Pattern::parse(
                &self.filter_string,
                CaseMatching::Ignore,
                Normalization::Smart,
            );

            // Re-fetch full directory and filter with match indices
            if let Ok(new_paths) = self.explorer.ls() {
                self.path_list = PathList::from_iter(new_paths);

                // Get all current items with their types
                let items_with_types: Vec<(String, ObjectType)> = self
                    .path_list
                    .items
                    .iter()
                    .map(|e| (e.value.to_string(), e.kind))
                    .collect();

                // Match and collect indices for each item
                let mut new_items = Vec::new();
                for (value, kind) in items_with_types {
                    let mut indices = Vec::new();
                    let mut buf = Vec::new();
                    let haystack = Utf32Str::new(&value, &mut buf);
                    let score = pattern.indices(haystack, &mut self.matcher, &mut indices);
                    if score.is_some() {
                        new_items.push((value, kind, indices, score.unwrap()));
                    }
                }

                // Sort by score (higher is better)
                new_items.sort_by(|a, b| b.3.cmp(&a.3));

                // Update path list with match indices
                self.path_list.items = new_items
                    .into_iter()
                    .map(|(value, kind, indices, _)| Path::with_match_indices(value, kind, indices))
                    .collect();
            }

            self.pattern = Some(pattern);
        }

        // Auto-select first item after backspace
        self.path_list.state.select_first();
    }

    fn update_command(&mut self, command: String, quit: bool) {
        if let Some(i) = self.path_list.state.selected() {
            self.output.command = command;
            let cwd = self.explorer.cwd();
            self.output.items = vec![self.path_list.items[i].value.to_string()]
                .iter()
                .map(|s| format!("{}/{}", cwd, s))
                .collect();
            if quit {
                self.should_exit = true;
            }
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
        App::render_footer(&self.filter_string, footer_area, buf);
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

    fn render_footer(filter_string: &str, area: Rect, buf: &mut Buffer) {
        let footer_text = if filter_string.is_empty() {
            "Use ↓↑ to move, ← to unselect, → to change status, g/G to go top/bottom.".to_string()
        } else {
            format!("Filter: {} | ESC to clear", filter_string)
        };
        Paragraph::new(footer_text).centered().render(area, buf);
    }

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

fn build_highlighted_line(value: &str, match_indices: &[u32]) -> Line<'static> {
    if match_indices.is_empty() {
        // No matches, render normally
        return Line::styled(value.to_string(), TEXT_FG_COLOR);
    }

    // Build spans with bold matching characters
    let mut spans = Vec::new();
    let chars: Vec<char> = value.chars().collect();
    let match_set: std::collections::HashSet<usize> =
        match_indices.iter().map(|&i| i as usize).collect();

    let mut current_text = String::new();
    let mut is_bold = false;

    for (idx, ch) in chars.iter().enumerate() {
        let should_be_bold = match_set.contains(&idx);

        if should_be_bold != is_bold {
            // Flush current segment
            if !current_text.is_empty() {
                if is_bold {
                    spans.push(ratatui::text::Span::styled(
                        current_text.clone(),
                        Style::default()
                            .fg(TEXT_FG_COLOR)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(ratatui::text::Span::styled(
                        current_text.clone(),
                        Style::default().fg(TEXT_FG_COLOR),
                    ));
                }
                current_text.clear();
            }
            is_bold = should_be_bold;
        }
        current_text.push(*ch);
    }

    // Flush remaining text
    if !current_text.is_empty() {
        if is_bold {
            spans.push(ratatui::text::Span::styled(
                current_text,
                Style::default()
                    .fg(TEXT_FG_COLOR)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(ratatui::text::Span::styled(
                current_text,
                Style::default().fg(TEXT_FG_COLOR),
            ));
        }
    }

    Line::from(spans)
}

impl From<&Path> for ListItem<'_> {
    fn from(path: &Path) -> Self {
        let line = build_highlighted_line(&path.value, &path.match_indices);
        ListItem::new(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_output_new() {
        let output = Output::new("/test/path".to_string());
        assert_eq!(output.cwd, "/test/path");
        assert_eq!(output.command, "no-op");
        assert_eq!(output.items.len(), 0);
    }

    #[test]
    fn test_output_display_no_items() {
        let output = Output::new("/test/path".to_string());
        assert_eq!(format!("{}", output), "/test/path no-op ");
    }

    #[test]
    fn test_output_display_with_single_item() {
        let mut output = Output::new("/test/path".to_string());
        output.command = "select".to_string();
        output.items = vec!["/test/path/file.txt".to_string()];
        assert_eq!(format!("{}", output), "/test/path select /test/path/file.txt");
    }

    #[test]
    fn test_output_display_with_multiple_items() {
        let mut output = Output::new("/test/path".to_string());
        output.command = "select".to_string();
        output.items = vec![
            "/test/path/file1.txt".to_string(),
            "/test/path/file2.txt".to_string(),
            "/test/path/file3.txt".to_string(),
        ];
        assert_eq!(
            format!("{}", output),
            "/test/path select /test/path/file1.txt /test/path/file2.txt /test/path/file3.txt"
        );
    }

    #[test]
    fn test_output_display_with_spaces_in_paths() {
        let mut output = Output::new("/test/path with spaces".to_string());
        output.command = "select".to_string();
        output.items = vec!["/test/path with spaces/file with spaces.txt".to_string()];
        assert_eq!(
            format!("{}", output),
            "/test/path with spaces select /test/path with spaces/file with spaces.txt"
        );
    }

    #[test]
    fn test_object_type_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path_buf = temp_dir.path().to_path_buf();
        let obj_type = ObjectType::from(path_buf);
        matches!(obj_type, ObjectType::Directory);
    }

    #[test]
    fn test_object_type_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "content").unwrap();
        let obj_type = ObjectType::from(file_path);
        matches!(obj_type, ObjectType::File);
    }

    #[test]
    fn test_path_new() {
        let path = Path::new("test.txt".to_string(), ObjectType::File);
        assert_eq!(path.value, "test.txt");
        matches!(path.kind, ObjectType::File);
    }

    #[test]
    fn test_pathlist_from_iter_pathbufs() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let paths: Vec<PathBuf> = vec![
            temp_dir.path().join("file1.txt"),
            temp_dir.path().join("file2.txt"),
            temp_dir.path().join("subdir"),
        ];

        let path_list = PathList::from_iter(paths);
        assert_eq!(path_list.items.len(), 3);
        assert_eq!(path_list.state.selected(), None);
    }

    #[test]
    fn test_pathlist_from_iter_dir_entries() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("alpha.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("beta.txt"), "content").unwrap();

        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|e| e.unwrap())
            .collect();

        let path_list = PathList::from_iter(entries);
        assert_eq!(path_list.items.len(), 2);

        let names: Vec<&str> = path_list
            .items
            .iter()
            .map(|p| p.value.as_str())
            .collect();
        assert!(names.contains(&"alpha.txt"));
        assert!(names.contains(&"beta.txt"));
    }

    #[test]
    fn test_pathlist_initial_state_no_selection() {
        let paths: Vec<PathBuf> = vec![];
        let path_list = PathList::from_iter(paths);
        assert_eq!(path_list.state.selected(), None);
    }

    #[test]
    fn test_app_select_first() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.select_first();
        assert_eq!(app.path_list.state.selected(), Some(0));
    }

    #[test]
    fn test_app_select_navigation() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file3.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.select_first();
        assert_eq!(app.path_list.state.selected(), Some(0));

        app.select_next();
        assert_eq!(app.path_list.state.selected(), Some(1));

        app.select_next();
        assert_eq!(app.path_list.state.selected(), Some(2));

        app.select_previous();
        assert_eq!(app.path_list.state.selected(), Some(1));

        app.select_previous();
        assert_eq!(app.path_list.state.selected(), Some(0));
    }

    #[test]
    fn test_app_select_none() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.select_first();
        assert_eq!(app.path_list.state.selected(), Some(0));
        app.select_none();
        assert_eq!(app.path_list.state.selected(), None);
    }

    #[test]
    fn test_app_update_command_with_selection() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd.clone()),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.select_first();
        app.update_command("test-cmd".to_string(), false);

        assert_eq!(app.output.command, "test-cmd");
        assert_eq!(app.output.items.len(), 1);
        assert!(app.output.items[0].ends_with("file.txt"));
        assert!(!app.should_exit);
    }

    #[test]
    fn test_app_update_command_with_quit() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.select_first();
        app.update_command("test-cmd".to_string(), true);

        assert!(app.should_exit);
    }

    #[test]
    fn test_app_update_command_without_selection() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.update_command("test-cmd".to_string(), false);

        assert_eq!(app.output.command, "no-op");
        assert_eq!(app.output.items.len(), 0);
    }

    #[test]
    fn test_app_clear_filter() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("alpha.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("beta.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.filter_string.push_str("alpha");
        app.clear_filter();

        assert_eq!(app.filter_string, "");
        assert!(app.pattern.is_none());
        assert_eq!(app.path_list.items.len(), 2);
        assert_eq!(app.path_list.state.selected(), Some(0));
    }

    #[test]
    fn test_app_filter_paths_single_match() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("alpha.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("beta.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("gamma.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.filter_paths('a');
        app.filter_paths('l');
        app.filter_paths('p');

        assert_eq!(app.filter_string, "alp");
        assert!(app.pattern.is_some());
        assert_eq!(app.path_list.items.len(), 1);
        assert_eq!(app.path_list.items[0].value, "alpha.txt");
        assert_eq!(app.path_list.state.selected(), Some(0));
    }

    #[test]
    fn test_app_filter_paths_multiple_matches() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("test2.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("other.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.filter_paths('t');
        app.filter_paths('e');

        assert!(app.path_list.items.len() >= 2);
        let names: Vec<&str> = app
            .path_list
            .items
            .iter()
            .map(|p| p.value.as_str())
            .collect();
        assert!(names.contains(&"test1.txt"));
        assert!(names.contains(&"test2.txt"));
    }

    #[test]
    fn test_app_filter_paths_no_matches() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("alpha.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("beta.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.filter_paths('x');
        app.filter_paths('y');
        app.filter_paths('z');

        assert_eq!(app.path_list.items.len(), 0);
    }

    #[test]
    fn test_app_remove_last_char_from_filter_empty_filter() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.remove_last_char_from_filter();
        assert_eq!(app.filter_string, "");
        assert!(app.pattern.is_none());
    }

    #[test]
    fn test_app_remove_last_char_from_filter_restores_full_list() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("alpha.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("beta.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        let initial_count = app.path_list.items.len();
        app.filter_paths('a');
        let filtered_count = app.path_list.items.len();
        assert!(filtered_count <= initial_count);

        app.remove_last_char_from_filter();
        assert_eq!(app.filter_string, "");
        assert!(app.pattern.is_none());
        assert_eq!(app.path_list.items.len(), initial_count);
    }

    #[test]
    fn test_app_remove_last_char_from_filter_with_remaining_chars() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("alpha.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("beta.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let cwd = explorer.cwd();
        let paths = explorer.ls().unwrap();
        let handle = stderr();

        let mut app = App {
            handle: &handle,
            should_exit: false,
            path_list: PathList::from_iter(paths),
            explorer: explorer,
            output: Output::new(cwd),
            matcher: Matcher::new(Config::DEFAULT.match_paths()),
            pattern: None,
            filter_string: String::new(),
        };

        app.filter_paths('a');
        app.filter_paths('l');
        app.filter_paths('p');
        assert_eq!(app.path_list.items.len(), 1);

        app.remove_last_char_from_filter();
        assert_eq!(app.filter_string, "al");
        assert!(app.pattern.is_some());
        assert_eq!(app.path_list.items.len(), 1);
    }

    #[test]
    fn test_build_highlighted_line_no_matches() {
        let line = build_highlighted_line("test.txt", &[]);
        let spans = &line.spans;

        // Should have exactly one span with all text, no bold
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "test.txt");
        assert!(!spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_build_highlighted_line_single_match_at_start() {
        let line = build_highlighted_line("test.txt", &[0]);
        let spans = &line.spans;

        // Should have 2 spans: bold 't' and normal 'est.txt'
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "t");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, "est.txt");
        assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_build_highlighted_line_consecutive_matches() {
        let line = build_highlighted_line("test.txt", &[0, 1, 2, 3]);
        let spans = &line.spans;

        // Should have 2 spans: bold "test" and normal ".txt"
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "test");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, ".txt");
        assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_build_highlighted_line_non_consecutive_matches() {
        let line = build_highlighted_line("test_file.txt", &[0, 5, 10]);
        let spans = &line.spans;

        // Should have 5 spans: bold 't', normal 'est_', bold 'f', normal 'ile.', bold 't', normal 'xt'
        assert_eq!(spans.len(), 6);
        assert_eq!(spans[0].content, "t");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, "est_");
        assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[2].content, "f");
        assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[3].content, "ile.");
        assert!(!spans[3].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[4].content, "t");
        assert!(spans[4].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[5].content, "xt");
        assert!(!spans[5].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_build_highlighted_line_all_matches() {
        let line = build_highlighted_line("abc", &[0, 1, 2]);
        let spans = &line.spans;

        // Should have 1 span with all bold
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "abc");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_build_highlighted_line_match_at_end() {
        let line = build_highlighted_line("test", &[3]);
        let spans = &line.spans;

        // Should have 2 spans: normal "tes" and bold "t"
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "tes");
        assert!(!spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, "t");
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_build_highlighted_line_empty_string() {
        let line = build_highlighted_line("", &[]);
        let spans = &line.spans;

        // Empty string produces 0 spans (no text to render)
        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn test_build_highlighted_line_alternating_matches() {
        let line = build_highlighted_line("abcd", &[0, 2]);
        let spans = &line.spans;

        // Should have 4 spans: bold 'a', normal 'b', bold 'c', normal 'd'
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].content, "a");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, "b");
        assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[2].content, "c");
        assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[3].content, "d");
        assert!(!spans[3].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_path_with_match_indices_constructor() {
        let indices = vec![0, 2, 4];
        let path = Path::with_match_indices(
            "hello.txt".to_string(),
            ObjectType::File,
            indices.clone(),
        );

        assert_eq!(path.value, "hello.txt");
        assert_eq!(path.match_indices, indices);
        matches!(path.kind, ObjectType::File);
    }

    #[test]
    fn test_path_default_constructor_has_empty_indices() {
        let path = Path::new("test.txt".to_string(), ObjectType::Directory);

        assert_eq!(path.value, "test.txt");
        assert_eq!(path.match_indices.len(), 0);
        matches!(path.kind, ObjectType::Directory);
    }
}
