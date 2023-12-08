use crate::{human_readable_folder_size, Message, PathItem, PathState};

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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

/// num of chars to preserve in path ellison
const PATH_PRESERVE_WIDTH: usize = 12;
/// interval to refresh ui
const TICK_INTERVAL: u64 = 100;
/// for separate path with kind text and size text
const PATH_SEPARATE: &str = " - ";
/// spinner dots
const SPINNER_DOTS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/// title or hint
const TITLE: &str = "Select with ↑CURSOR↓ and press SPACE key to delete.";

pub fn run(rx: Receiver<Message>, tx: Sender<Message>) -> Result<()> {
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
    tx: Sender<Message>,
    rx: Receiver<Message>,
    mut app: App,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(TICK_INTERVAL);

    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Ok(item) = rx.try_recv() {
            match item {
                Message::AddPath(item) => {
                    app.total_size += item.size.unwrap_or_default();
                    app.add_item(item);
                }
                Message::DoneSearch => {
                    app.done_search = true;
                }
                Message::SetPathDeleted(path) => {
                    let size = app.set_item_deleted(path);
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
                let mut set_last_code = true;

                app.clear_tmp_state();

                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Char('?') => app.show_help = true,
                    KeyCode::Home => app.begin(),
                    KeyCode::Char('G') | KeyCode::End => app.end(),
                    KeyCode::Char('g') => {
                        if let Some(KeyCode::Char('g')) = app.last_keycode {
                            app.begin();
                            set_last_code = false;
                        }
                    }
                    KeyCode::Char('p') => {
                        if let Some(KeyCode::Char('o')) = app.last_keycode {
                            app.order_by_path();
                            set_last_code = false;
                        }
                    }
                    KeyCode::Char('s') => {
                        if let Some(KeyCode::Char('o')) = app.last_keycode {
                            app.order_by_size();
                            set_last_code = false;
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(KeyCode::Char('d')) = app.last_keycode {
                            app.delete_item(tx.clone());
                            set_last_code = false;
                        }
                    }
                    KeyCode::Char('a') => {
                        if let Some(KeyCode::Char('d')) = app.last_keycode {
                            app.delete_all_items(tx.clone());
                            set_last_code = false;
                        }
                    }
                    KeyCode::Char(' ') => {
                        app.delete_item(tx.clone());
                        app.last_keycode = None;
                    }
                    _ => {}
                }
                if set_last_code {
                    app.last_keycode = Some(key.code);
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
    if app.show_help {
        draw_help_view(f, f.size());
        return;
    }

    let constraints = if app.error.is_some() {
        vec![
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Min(0), Constraint::Length(1)]
    };

    let chunks = Layout::default().constraints(constraints).split(f.size());

    draw_list_view(f, app, chunks[0]);
    draw_status_bar(f, app, chunks[1]);
    if let Some(error) = app.error.as_ref() {
        draw_error_line(f, error, chunks[2])
    }
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
            width -= (item.size_text.len() + PATH_SEPARATE.len()) as u16;
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
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(16)].as_ref())
        .split(area);

    let indicator = if app.done_search {
        " ✔ ".to_string()
    } else {
        format!(" {} ", app.spinner())
    };
    let spans = vec![
        Span::raw(indicator),
        Span::styled("total space:", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {} ", human_readable_folder_size(app.total_size))),
        Span::styled("released space:", Style::default().fg(Color::DarkGray)),
        Span::raw(format!(
            " {} ",
            human_readable_folder_size(app.total_saved_size)
        )),
    ];
    let status_text = Paragraph::new(Spans::from(spans));

    let spans = vec![Span::styled(
        "Press ? for help".to_string(),
        Style::default().fg(Color::DarkGray),
    )];

    let help_text = Paragraph::new(Spans::from(spans));

    f.render_widget(status_text, chunks[0]);
    f.render_widget(help_text, chunks[1]);
}

fn draw_error_line<B: Backend>(f: &mut Frame<B>, error: &str, area: Rect) {
    let paragraph = Paragraph::new(Spans::from(vec![Span::styled(
        error.to_string(),
        Style::default().fg(Color::Red),
    )]));
    f.render_widget(paragraph, area);
}

fn draw_help_view<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let help_docs = vec![
        ["Move selection up", "k  | <up> "],
        ["Move selection down", "j  | <down> "],
        ["Move to the top", "gg | <home> "],
        ["Move to the bottom", "G  | <end> "],
        ["Delete selected folder", "dd | <space> "],
        ["Delete all listed folder", "da"],
        ["Sort by path", "op"],
        ["Sort by size", "os"],
        ["Exit", "q  | <ctrl+c>"],
    ];

    let items: Vec<ListItem> = help_docs
        .into_iter()
        .map(|row| {
            let [desc, keycode] = row;
            let desc_style = Style::default();
            let keycode_style = Style::default();
            let content = vec![Spans::from(vec![
                Span::styled(format!(" {desc:<30}"), desc_style),
                Span::styled(keycode.to_string(), keycode_style),
            ])];
            ListItem::new(content)
        })
        .collect();

    let title = Span::styled(" Help ", Style::default().fg(Color::Yellow));
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Spans::from(vec![title])),
    );
    f.render_widget(list, area);
}

#[derive(Debug, Default)]
struct App {
    state: ListState,
    items: Vec<PathItem>,
    spinner_index: usize,
    total_size: u64,
    total_saved_size: u64,
    done_search: bool,
    show_help: bool,
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
            spawn_delete_path(path, sender);
        }
    }

    fn delete_all_items(&mut self, sender: Sender<Message>) {
        for item in self.items.iter_mut() {
            if item.state == PathState::Normal && item.size.is_some() {
                item.state = PathState::StartDeleting;
                spawn_delete_path(item.path.clone(), sender.clone());
            }
        }
    }

    fn start_deleting_item(&mut self) -> Option<PathBuf> {
        if let Some(index) = self.state.selected() {
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
        self.show_help = false;
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

fn spawn_delete_path(path: PathBuf, sender: Sender<Message>) {
    thread::spawn(move || match remove_dir_all(&path) {
        Ok(_) => sender.send(Message::SetPathDeleted(path)).unwrap(),
        Err(err) => sender
            .send(Message::PutError(format!(
                "Cannot delete '{}', {}",
                path.display(),
                err
            )))
            .unwrap(),
    });
}
