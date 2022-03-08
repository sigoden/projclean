use crate::Event;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as TuiEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::path::{Path, PathBuf};
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
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame, Terminal,
};

static UNITS: [char; 4] = ['T', 'G', 'M', 'K'];
/// limit to 16 chars
const KIND_WIDTH: usize = 12;
/// interval to refresh ui
const TICK_INTERVAL: u64 = 100;

const LOADING_SPINNER_DOTS: [&str; 4] = ["◐", "◓", "◑", "◒"];

pub fn run(rx: Receiver<Event>) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let table_view = TableView::default();
    let status_bar = StatusBar::default();
    let res = run_ui(&mut terminal, rx, table_view, status_bar);

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
struct TableView {
    state: TableState,
    items: Vec<PathItem>,
}

impl TableView {
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
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
                    self.items.len() - 1
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
}

fn run_ui<B: Backend>(
    terminal: &mut Terminal<B>,
    receiver: Receiver<Event>,
    mut table_view: TableView,
    mut status_bar: StatusBar,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(TICK_INTERVAL);

    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw(f, &mut table_view, &mut status_bar))?;
        if let Ok(item) = receiver.try_recv() {
            match item {
                Event::SearchFoundPath(item) => {
                    status_bar.total_size += item.size.unwrap_or_default();
                    table_view.add_item(item);
                }
                Event::SearchFinished => {
                    status_bar.is_finished_search = true;
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
                    KeyCode::Char('j') | KeyCode::Down => table_view.next(),
                    KeyCode::Char('k') | KeyCode::Up => table_view.previous(),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            status_bar.on_tick();
            last_tick = Instant::now();
        }
    }
}

#[derive(Debug, Default)]
struct StatusBar {
    spinner_index: usize,
    total_size: u64,
    total_saved_size: u64,
    is_finished_search: bool,
}

impl StatusBar {
    fn indicator(&self) -> Span {
        if self.is_finished_search {
            Span::raw(" ✔ ".to_string())
        } else {
            let dot = LOADING_SPINNER_DOTS[self.spinner_index];
            Span::raw(format!(" {} ", dot))
        }
    }
    fn on_tick(&mut self) {
        self.spinner_index = (self.spinner_index + 1) % LOADING_SPINNER_DOTS.len()
    }
}

fn draw<B: Backend>(f: &mut Frame<B>, table_view: &mut TableView, status_bar: &mut StatusBar) {
    let chunks = Layout::default()
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(f.size());

    draw_table_view(f, table_view, chunks[0]);
    draw_status_bar(f, status_bar, chunks[1]);
}

fn draw_table_view<B: Backend>(f: &mut Frame<B>, table_view: &mut TableView, area: Rect) {
    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let header = Row::new(PathItem::header_cells()).height(1);
    let rows = table_view.items.iter().map(|item| {
        let cells = item.row_cells();
        Row::new(cells).height(1)
    });
    let widths = PathItem::widths();
    let table = Table::new(rows)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Project Cleaner "),
        )
        .highlight_style(selected_style)
        .widths(&widths);
    f.render_stateful_widget(table, area, &mut table_view.state);
}

fn draw_status_bar<B: Backend>(f: &mut Frame<B>, status_bar: &mut StatusBar, area: Rect) {
    let spans = Spans::from(vec![
        status_bar.indicator(),
        Span::styled("releasable space: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(
            "{} ",
            human_readable_folder_size(status_bar.total_size)
        )),
        Span::styled("saved space: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(
            "{} ",
            human_readable_folder_size(status_bar.total_saved_size)
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
pub struct PathItem {
    /// Project kind
    pub kind: String,
    /// Path
    pub path: PathBuf,
    /// Total storage size
    pub size: Option<u64>,
}

impl PathItem {
    pub fn new(kind: &str, path: &Path, size: Option<u64>) -> Self {
        PathItem {
            kind: Self::truncate_kind(kind),
            path: path.to_path_buf(),
            size,
        }
    }
    pub fn widths() -> Vec<Constraint> {
        vec![Constraint::Percentage(90), Constraint::Min(10)]
    }
    pub fn header_cells() -> Vec<Cell<'static>> {
        vec![
            Cell::from("Select with ↑CURSOR↓ and press SPACE key to delete ⚠")
                .style(Style::default().fg(Color::Yellow)),
            Cell::from("space").style(Style::default().fg(Color::DarkGray)),
        ]
    }
    pub fn row_cells(&self) -> Vec<Cell> {
        vec![
            Cell::from(self.path.to_string_lossy()),
            Cell::from(match self.size {
                Some(size) => human_readable_folder_size(size),
                None => "?".to_string(),
            }),
        ]
    }
    fn truncate_kind(kind: &str) -> String {
        if kind.len() <= KIND_WIDTH {
            kind.to_string()
        } else {
            kind[0..KIND_WIDTH].to_string()
        }
    }
}
