use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    fs::remove_dir_all,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
};
use std::{
    io,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

/// storage space unit
static UNITS: [char; 4] = ['T', 'G', 'M', 'K'];
/// limit kind string to 16 chars
const KIND_LIMIT_WIDTH: usize = 12;
/// num of chars to preserve in path ellision
const PATH_PRESERVE_WIDTH: usize = 12;
/// interval to refresh ui
const TICK_INTERVAL: u64 = 100;
/// for separate path with kind text and size text
const PATH_SEPARATE: &str = " - ";
/// spinner dots
const SPINNER_DOTS: [&str; 4] = ["◐", "◓", "◑", "◒"];
/// title or hint
const TITLE: &str = "Select with ↑CURSOR↓ and press SPACE key to delete ⚠";

pub fn run(tx: Sender<Message>, rx: Receiver<Message>) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::default();
    let res = run_ui(&mut terminal, tx, rx, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res?;

    Ok(())
}

fn run_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    mut app: App,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(TICK_INTERVAL);

    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Ok(item) = receiver.try_recv() {
            match item {
                Message::AddPath(item) => {
                    app.total_size += item.size.unwrap_or_default();
                    app.add_item(item);
                }
                Message::DoneSearch => {
                    app.is_done = true;
                }
                Message::SetPathDeleted(index) => {
                    let size = app.set_item_deleted(index);
                    app.total_saved_size += size.unwrap_or_default();
                }
                Message::PutError(message) => {
                    app.error = Some(message);
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.clear_error();

                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Home => app.begin(),
                    KeyCode::Char('G') | KeyCode::End => app.end(),
                    KeyCode::Char('g') => {
                        if let Some(KeyCode::Char('g')) = app.last_keycode {
                            app.begin();
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some((index, path)) = app.start_deleting_item() {
                            let sender = sender.clone();
                            thread::spawn(move || match remove_dir_all(&path) {
                                Ok(_) => sender.send(Message::SetPathDeleted(index)).unwrap(),
                                Err(err) => sender
                                    .send(Message::PutError(format!(
                                        "Cannot delete '{}', {}",
                                        path.display(),
                                        err
                                    )))
                                    .unwrap(),
                            });
                        }
                    }
                    _ => {}
                }
                app.last_keycode = Some(key.code);
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let height = if app.error.is_some() { 2 } else { 1 };
    let chunks = Layout::default()
        .constraints([Constraint::Min(0), Constraint::Length(height)].as_ref())
        .split(f.size());

    draw_list_view(f, app, chunks[0]);
    draw_status_bar(f, app, chunks[1]);
}

fn draw_list_view<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let width = area.width - 2;
    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let is_selected = app
                .state
                .selected()
                .map(|selected| selected == index)
                .unwrap_or_default();

            let mut width = width;
            width -= (item.kind_text.len() + item.size_text.len() + PATH_SEPARATE.len()) as u16;
            let mut styles = vec![
                Style::default(),
                Style::default(),
                Style::default().fg(Color::DarkGray),
            ];
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
                    Span::styled(format!(" {} ", app.spinner()), styles[0])
                }
                _ => Span::styled("", styles[0]),
            };
            let path_span = Span::styled(truncate_path(&item.relative_path, width), styles[0]);
            let separate_span = Span::styled(PATH_SEPARATE, styles[0]);
            let size_span = Span::styled(item.size_text.clone(), styles[1]);
            let mut spans = vec![path_span, separate_span, size_span];
            if !item.kind_text.is_empty() {
                let kind_span = Span::styled(item.kind_text.clone(), styles[2]);
                spans.push(kind_span);
            }
            spans.push(indicator_span);
            ListItem::new(Spans::from(spans))
        })
        .collect();
    let title = Span::styled(TITLE, Style::default().fg(Color::Yellow));
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Spans::from(vec![title])),
    );
    f.render_stateful_widget(list, area, &mut app.state);
}

fn draw_status_bar<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let mut text = vec![Spans::from(vec![
        Span::raw(format!("{} ", app.status_bar_indicator())),
        Span::styled("releasable space:", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {} ", human_readable_folder_size(app.total_size))),
        Span::styled("saved space:", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(
            " {} ",
            human_readable_folder_size(app.total_saved_size)
        )),
    ])];
    if let Some(message) = &app.error {
        text.push(Spans::from(vec![Span::styled(
            format!("error: {} ", message),
            Style::default().fg(Color::Red),
        )]));
    }
    let paragraph = Paragraph::new(text);
    f.render_widget(paragraph, area);
}

fn human_readable_folder_size(size: u64) -> String {
    for (i, u) in UNITS.iter().enumerate() {
        let num: u64 = 1024;
        let marker = num.pow((UNITS.len() - i) as u32);
        if size >= marker {
            if size / marker < 10 {
                return format!("{:.1}{}", (size as f32 / marker as f32), u);
            } else {
                return format!("{}{}", (size / marker), u);
            }
        }
    }
    return format!("{}B", size);
}

#[derive(Debug)]
pub enum Message {
    AddPath(PathItem),
    SetPathDeleted(usize),
    PutError(String),
    DoneSearch,
}

#[derive(Debug, Default)]
struct App {
    state: ListState,
    items: Vec<PathItem>,
    spinner_index: usize,
    total_size: u64,
    total_saved_size: u64,
    is_done: bool,
    error: Option<String>,
    last_keycode: Option<KeyCode>,
}

impl App {
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn begin(&mut self) {
        let len = self.items.len();
        if len == 0 {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }

    fn end(&mut self) {
        let len = self.items.len();
        if len == 0 {
            self.state.select(None);
        } else {
            self.state.select(Some(len - 1));
        }
    }

    fn add_item(&mut self, item: PathItem) {
        self.items.push(item);
    }

    fn start_deleting_item(&mut self) -> Option<(usize, PathBuf)> {
        if let Some(index) = self.state.selected() {
            let item = &mut self.items[index];
            if item.state != PathState::Normal || item.size.is_none() {
                None
            } else {
                item.state = PathState::StartDeleting;
                Some((index, item.path.clone()))
            }
        } else {
            None
        }
    }

    fn set_item_deleted(&mut self, index: usize) -> Option<u64> {
        if let Some(item) = self.items.get_mut(index) {
            item.state = PathState::Deleted;
            item.size
        } else {
            None
        }
    }

    fn status_bar_indicator(&self) -> &'static str {
        if self.is_done {
            "✔"
        } else {
            self.spinner()
        }
    }

    fn spinner(&self) -> &'static str {
        SPINNER_DOTS[self.spinner_index]
    }

    fn clear_error(&mut self) {
        if self.error.is_some() {
            self.error = None;
        }
    }

    fn on_tick(&mut self) {
        self.spinner_index = (self.spinner_index + 1) % SPINNER_DOTS.len()
    }
}

#[derive(Debug)]
pub struct PathItem {
    path: PathBuf,
    relative_path: PathBuf,
    size: Option<u64>,
    size_text: String,
    kind_text: String,
    state: PathState,
}

#[derive(Debug, PartialEq)]
enum PathState {
    Normal,
    StartDeleting,
    Deleted,
}

impl PathItem {
    pub fn new(
        path: PathBuf,
        relative_path: PathBuf,
        size: Option<u64>,
        kind: Option<String>,
    ) -> Self {
        let size_text = match size {
            Some(size) => format!("[{}]", human_readable_folder_size(size)),
            None => "[?]".to_string(),
        };
        let kind_text = match kind.as_ref() {
            Some(kind) => format!("({})", truncate_kind(kind)),
            None => "".to_string(),
        };
        PathItem {
            path,
            relative_path,
            size,
            size_text,
            kind_text,
            state: PathState::Normal,
        }
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
fn truncate_kind(kind: &str) -> String {
    if kind.len() <= KIND_LIMIT_WIDTH {
        kind.to_string()
    } else {
        kind[0..KIND_LIMIT_WIDTH].to_string()
    }
}
