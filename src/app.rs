use crate::{human_readable_folder_size, Message, PathItem, PathState};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use remove_dir_all::remove_dir_all;
use std::{
    io::{self, stdout},
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};
use threadpool::ThreadPool;

/// num of chars to preserve in path ellison
const PATH_PRESERVE_WIDTH: usize = 12;
/// interval to refresh ui
const TICK_INTERVAL: u64 = 100;
/// for separate path with kind text and size text
const PATH_SEPARATE: &str = " - ";
/// spinner dots
const SPINNER_DOTS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[derive(Debug, Default)]
struct App {
    list_state: ListState,
    items: Vec<PathItem>,
    spinner_index: usize,
    total_size: u64,
    total_saved_size: u64,
    error: Option<String>,
    app_state: AppState,
    pool: ThreadPool,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
enum AppState {
    #[default]
    Searching,
    SearchingDone,
    Exit,
}

pub fn run(rx: Receiver<Message>, tx: Sender<Message>) -> io::Result<()> {
    let mut terminal = init_terminal()?;
    // result is evaluated after restoring terminal to ensure that it does not get printed on the
    // alternate screen in raw mode
    let res = App::default().run(&mut terminal, tx, rx);
    restore_terminal(terminal)?;
    res
}

fn init_terminal() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

fn restore_terminal(mut terminal: Terminal<impl Backend>) -> io::Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

impl App {
    fn run(
        mut self,
        terminal: &mut Terminal<impl Backend>,
        tx: Sender<Message>,
        rx: Receiver<Message>,
    ) -> io::Result<()> {
        let tick_rate = Duration::from_millis(TICK_INTERVAL);
        let mut last_tick = Instant::now();
        while self.app_state != AppState::Exit {
            terminal.draw(|frame| self.draw(frame))?;

            self.handle_next_message(&rx);

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            self.handle_events(timeout, &tx)?;

            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn handle_next_message(&mut self, rx: &Receiver<Message>) {
        let Ok(item) = rx.try_recv() else { return };
        match item {
            Message::AddPath(item) => {
                self.total_size += item.size.unwrap_or_default();
                self.add_item(item);
            }
            Message::DoneSearch => {
                self.app_state = AppState::SearchingDone;
            }
            Message::SetPathDeleted(path) => {
                let size = self.set_item_deleted(path);
                self.total_saved_size += size.unwrap_or_default();
            }
            Message::PutError(message) => {
                self.error = Some(message);
            }
        }
    }

    fn handle_events(&mut self, timeout: Duration, tx: &Sender<Message>) -> Result<(), io::Error> {
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key, tx)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: event::KeyEvent, tx: &Sender<Message>) -> io::Result<()> {
        // ignore key release event
        if key.kind != event::KeyEventKind::Press {
            return Ok(());
        }
        self.clear_tmp_state();
        match key.code {
            KeyCode::Down => {
                if key.kind == event::KeyEventKind::Press {
                    self.next()
                }
            }
            KeyCode::Up => {
                if key.kind == event::KeyEventKind::Press {
                    self.previous()
                }
            }
            KeyCode::Char(' ') => {
                self.delete_item(tx.clone());
            }
            KeyCode::Home => self.begin(),
            KeyCode::End => self.end(),
            KeyCode::F(4) => self.delete_all_items(tx.clone()),
            KeyCode::F(7) => self.order_by_path(),
            KeyCode::F(8) => self.order_by_size(),
            KeyCode::Esc => {
                self.app_state = AppState::Exit;
            }
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                self.app_state = AppState::Exit;
            }
            _ => {}
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let mut constraints = vec![Constraint::Min(0), Constraint::Length(1)];
        if self.error.is_some() {
            constraints.push(Constraint::Length(1));
        };

        let areas = Layout::default()
            .constraints(constraints)
            .split(frame.size());

        self.draw_list_view(frame, areas[0]);
        self.draw_status_bar(frame, areas[1]);
        if let Some(error) = self.error.as_ref() {
            Self::draw_error_line(frame, error, areas[2])
        }
    }

    fn draw_list_view(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let is_selected = self.list_state.selected() == Some(index);
                let mut width = area.width - 2;
                width -= (item.size_text.len() + PATH_SEPARATE.len()) as u16;
                let mut styles = vec![Style::default(), Style::default()];
                if is_selected {
                    styles = styles.into_iter().map(|v| v.fg(Color::Cyan)).collect();
                }
                let indicator_span = match item.state {
                    PathState::Deleted => {
                        styles = styles
                            .into_iter()
                            .map(|v| v.add_modifier(Modifier::DIM))
                            .collect();
                        width -= 3;
                        Span::styled(" ✘ ", styles[0])
                    }
                    PathState::StartDeleting => {
                        width -= 3;
                        Span::styled(format!(" {} ", self.spinner()), styles[0])
                    }
                    _ => Span::styled("", styles[0]),
                };
                let path_span = Span::styled(truncate_path(&item.relative_path, width), styles[0]);
                let separate_span = Span::styled(PATH_SEPARATE, styles[0]);
                let size_span = Span::styled(item.size_text.clone(), styles[1]);
                let mut spans = vec![path_span, separate_span, size_span];
                spans.push(indicator_span);
                ListItem::new(Line::from(spans))
            })
            .collect();
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Self::title_line()),
        );
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn draw_status_bar(&mut self, frame: &mut Frame, area: Rect) {
        let search_indicator = match self.app_state {
            AppState::Searching => format!(" {} ", self.spinner()),
            AppState::SearchingDone => " ✔ ".to_string(),
            AppState::Exit => " ✘ ".to_string(),
        };

        let status_line = Line::from(vec![
            search_indicator.into(),
            "total space: ".dark_gray(),
            human_readable_folder_size(self.total_size).into(),
            " released space:".dark_gray(),
            human_readable_folder_size(self.total_saved_size).into(),
            " ".into(),
        ]);

        frame.render_widget(Paragraph::new(status_line), area);
    }

    fn draw_error_line(frame: &mut Frame, error: &str, area: Rect) {
        let error_line = error.to_string().red();
        frame.render_widget(Paragraph::new(error_line), area);
    }

    fn title_line() -> Line<'static> {
        let hotkeys = vec![
            ("↑↓", "Move"),
            ("SPACE", "Delete"),
            ("F4", "Delete All"),
            ("F7/F8", "Sort by Path/Size"),
            ("ESC", "Exit"),
        ];
        let colors = [
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::DarkGray),
            Style::default(),
        ];
        let spans: Vec<Span<'static>> = hotkeys
            .into_iter()
            .map(|(k, v)| {
                vec![
                    Span::styled(format!(" {k} "), colors[0]),
                    Span::styled(format!("{v} "), colors[1]),
                ]
            })
            .collect::<Vec<Vec<_>>>()
            .join(&Span::styled("|", colors[2]));
        Line::from(spans)
    }
}

impl App {
    /// move selection to next item (with wrap around to the top)
    fn next(&mut self) {
        let next = self
            .list_state
            .selected()
            .map(|i| (i + 1) % self.items.len())
            .or(Some(0));
        self.list_state.select(next);
    }

    /// select the previous item (with wrap around to the bottom)
    fn previous(&mut self) {
        let next = self
            .list_state
            .selected()
            .map(|i| (i + self.items.len().saturating_sub(1)) % self.items.len())
            .or(Some(0));
        self.list_state.select(next);
    }

    /// move selection to the top
    fn begin(&mut self) {
        if self.items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn end(&mut self) {
        if self.items.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(self.items.len() - 1));
        }
    }

    fn order_by_path(&mut self) {
        self.items
            .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    }

    fn order_by_size(&mut self) {
        self.items
            .sort_by(|b, a| a.size.unwrap_or_default().cmp(&b.size.unwrap_or_default()));
    }

    fn add_item(&mut self, item: PathItem) {
        self.items.push(item);
    }

    fn delete_item(&mut self, sender: Sender<Message>) {
        if let Some(path) = self.start_deleting_item() {
            spawn_delete_path(self.pool.clone(), path, sender);
        }
    }

    fn delete_all_items(&mut self, sender: Sender<Message>) {
        for item in self.items.iter_mut() {
            if item.state == PathState::Normal && item.size.is_some() {
                item.state = PathState::StartDeleting;
                spawn_delete_path(self.pool.clone(), item.path.clone(), sender.clone());
            }
        }
    }

    fn start_deleting_item(&mut self) -> Option<PathBuf> {
        if let Some(index) = self.list_state.selected() {
            let item = &mut self.items[index];
            if item.state != PathState::Normal || item.size.is_none() {
                None
            } else {
                item.state = PathState::StartDeleting;
                Some(item.path.clone())
            }
        } else {
            None
        }
    }

    fn set_item_deleted(&mut self, path: PathBuf) -> Option<u64> {
        if let Some(item) = self.items.iter_mut().find(|item| item.path == path) {
            item.state = PathState::Deleted;
            item.size
        } else {
            None
        }
    }

    fn spinner(&self) -> &'static str {
        SPINNER_DOTS[self.spinner_index]
    }

    fn clear_tmp_state(&mut self) {
        if self.error.is_some() {
            self.error = None;
        }
    }

    fn on_tick(&mut self) {
        self.spinner_index = (self.spinner_index + 1) % SPINNER_DOTS.len()
    }
}

fn truncate_path(path: &Path, width: u16) -> String {
    let path = path.to_string_lossy();
    let preserve_len: usize = PATH_PRESERVE_WIDTH;
    let width = (width as usize).max(2 * preserve_len + 3);
    let len = path.len();
    if len <= width {
        return path.to_string();
    }
    format!(
        "{}...{}",
        &path[0..preserve_len],
        &path[(len - width + preserve_len + 3)..]
    )
}

fn spawn_delete_path(pool: ThreadPool, path: PathBuf, sender: Sender<Message>) {
    pool.execute(move || delete_path(path, sender));
}

fn delete_path(path: PathBuf, sender: Sender<Message>) {
    match remove_dir_all(&path) {
        Ok(_) => sender.send(Message::SetPathDeleted(path)).unwrap(),
        Err(err) => {
            let msg = Message::PutError(format!("Cannot delete '{}', {}", path.display(), err));
            sender.send(msg).unwrap()
        }
    }
}
