use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as TuiEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{error, warn};
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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

static UNITS: [char; 4] = ['T', 'G', 'M', 'K'];
/// limit to 16 chars
const KIND_WIDTH: usize = 12;
/// interval to refresh ui
const TICK_INTERVAL: u64 = 100;

const LOADING_SPINNER_DOTS: [&str; 4] = ["◐", "◓", "◑", "◒"];
const TITLE: &str = "Select with ↑CURSOR↓ and press SPACE key to delete ⚠";

pub fn run(tx: Sender<Event>, rx: Receiver<Event>) -> Result<()> {
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

#[derive(Debug, Default)]
struct App {
    state: ListState,
    items: Vec<PathItem>,
    spinner_index: usize,
    total_size: u64,
    total_saved_size: u64,
    is_done: bool,
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

    fn add_item(&mut self, item: PathItem) {
        self.items.push(item);
    }

    fn start_deleting_item(&mut self) -> Option<(usize, PathBuf)> {
        if let Some(index) = self.state.selected() {
            let item = &mut self.items[index];
            if item.state != PathState::Normal {
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
        LOADING_SPINNER_DOTS[self.spinner_index]
    }
    fn on_tick(&mut self) {
        self.spinner_index = (self.spinner_index + 1) % LOADING_SPINNER_DOTS.len()
    }
}

fn run_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    sender: Sender<Event>,
    receiver: Receiver<Event>,
    mut app: App,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(TICK_INTERVAL);

    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Ok(item) = receiver.try_recv() {
            match item {
                Event::FoundPath(item) => {
                    app.total_size += item.size.unwrap_or_default();
                    app.add_item(item);
                }
                Event::SearchFinished => {
                    app.is_done = true;
                }
                Event::DeletedPath(index) => {
                    let size = app.set_item_deleted(index);
                    app.total_saved_size += size.unwrap_or_default();
                }
            }
        }

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let TuiEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Char(' ') => {
                        if let Some((index, path)) = app.start_deleting_item() {
                            let sender = sender.clone();
                            thread::spawn(move || {
                                if let Err(err) = remove_dir_all(&path) {
                                    error!("Fail to remove dir {}, {}", path.display(), err);
                                }
                                if let Err(err) = sender.send(Event::DeletedPath(index)) {
                                    warn!("Fail to send deleted path index {}, {}", index, err);
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
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
            let size_text = match item.size {
                Some(size) => human_readable_folder_size(size),
                None => "?".to_string(),
            };
            width -= (item.kind.len() + size_text.len() + 7) as u16;
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
                        .map(|v| v.add_modifier(Modifier::CROSSED_OUT))
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
            let path_span = Span::styled(truncate_path(&item.path, width), styles[0]);
            let sperate_span = Span::styled(" - ", styles[0]);
            let size_span = Span::styled(format!("[{}]", size_text,), styles[1]);
            let kind_span = Span::styled(format!("({})", item.kind), styles[2]);
            ListItem::new(Spans::from(vec![
                indicator_span,
                path_span,
                sperate_span,
                size_span,
                kind_span,
            ]))
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
    let spans = Spans::from(vec![
        Span::raw(format!(" {} ", app.status_bar_indicator())),
        Span::styled("releasable space: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{} ", human_readable_folder_size(app.total_size))),
        Span::styled("saved space: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(
            "{} ",
            human_readable_folder_size(app.total_saved_size)
        )),
    ]);
    let paragraph = Paragraph::new(vec![spans]).wrap(Wrap { trim: true });
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
pub enum Event {
    FoundPath(PathItem),
    DeletedPath(usize),
    SearchFinished,
}

#[derive(Debug)]
pub struct PathItem {
    kind: String,
    path: PathBuf,
    size: Option<u64>,
    state: PathState,
}

#[derive(Debug, PartialEq)]
enum PathState {
    Normal,
    StartDeleting,
    Deleted,
}

impl PathItem {
    pub fn new(kind: &str, path: &Path, size: Option<u64>) -> Self {
        PathItem {
            kind: truncate_kind(kind),
            path: path.to_path_buf(),
            size,
            state: PathState::Normal,
        }
    }
}

fn truncate_path(path: &Path, width: u16) -> String {
    let path = path.to_string_lossy();
    let perserve_len: usize = 12;
    let width = (width as usize).max(2 * perserve_len + 3);
    let len = path.len();
    if len <= width {
        return path.to_string();
    }
    format!(
        "{}...{}",
        &path[0..perserve_len],
        &path[(len - width + perserve_len + 3)..]
    )
}
fn truncate_kind(kind: &str) -> String {
    if kind.len() <= KIND_WIDTH {
        kind.to_string()
    } else {
        kind[0..KIND_WIDTH].to_string()
    }
}
