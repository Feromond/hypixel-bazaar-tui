use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::time::{self, Duration};
use tokio::sync::mpsc;

use crate::app::state::{App, View, SearchMode};
use crate::api::models::Product;
use crate::ui::views::{draw_detail, draw_search};

pub async fn run_app(app: &mut App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // channel for background updates from refresh task
    let (tx, rx) = mpsc::unbounded_channel();
    app.set_update_sender(tx);
    
    let res = run_loop(app, &mut terminal, rx).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res
}

async fn run_loop(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    mut rx: mpsc::UnboundedReceiver<Product>,
) -> io::Result<()> {
    let mut tick = time::interval(Duration::from_millis(60));
    let debounce = Duration::from_millis(120);

    loop {
        terminal.draw(|f| match app.view {
            View::Search => draw_search(f, app),
            View::Detail => draw_detail(f, app),
        })?;

        tokio::select! {
            _ = tick.tick() => {
                // Debounced filter on search input
                if app.view == View::Search {
                    app.maybe_apply_filter(debounce);
                }
            }
            Some(p) = rx.recv() => {
                app.update_product(p);
            }
            Ok(should_quit) = handle_event(app) => {
                if should_quit { break; }
            }
        }
    }
    Ok(())
}

async fn handle_event(app: &mut App) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(16))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                // Global quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(true);
                }

                return Ok(match app.view {
                    View::Search => handle_search_input(app, key),
                    View::Detail => handle_detail_input(app, key),
                });
            }
        }
    }
    Ok(false)
}

fn handle_search_input(app: &mut App, key: event::KeyEvent) -> bool {
    match app.search.mode {
        SearchMode::Insert => match key.code {
            KeyCode::Esc => {
                if app.search.input.is_empty() {
                    return true; // quit
                } else {
                    app.on_delete();
                }
            }
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.jump_to_top();
                } else {
                    app.move_selection(-1);
                }
                app.search.mode = SearchMode::Navigate;
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.jump_to_bottom();
                } else {
                    app.move_selection(1);
                }
                app.search.mode = SearchMode::Navigate;
            }
            KeyCode::PageUp => {
                app.move_selection(-20);
                app.search.mode = SearchMode::Navigate;
            }
            KeyCode::PageDown => {
                app.move_selection(20);
                app.search.mode = SearchMode::Navigate;
            }
            KeyCode::Home => {
                app.jump_to_top();
                app.search.mode = SearchMode::Navigate;
            }
            KeyCode::End => {
                app.jump_to_bottom();
                app.search.mode = SearchMode::Navigate;
            }
            KeyCode::Backspace => app.on_backspace(),
            KeyCode::Delete => app.on_delete(),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.search.sort_by_spread = !app.search.sort_by_spread;
                app.recompute_filter();
                app.status = if app.search.sort_by_spread { "Sorted by spread".into() } else { "Sorted by relevance".into() };
            }
            KeyCode::Char(ch) => app.on_input(ch),
            KeyCode::Enter => app.enter_detail(),
            _ => {}
        },
        SearchMode::Navigate => match key.code {
            KeyCode::Esc => {
                if app.search.input.is_empty() {
                    return true;
                } else {
                    app.on_delete();
                }
            }
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.jump_to_top();
                } else {
                    app.move_selection(-1);
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.jump_to_bottom();
                } else {
                    app.move_selection(1);
                }
            }
            KeyCode::PageUp => app.move_selection(-20),
            KeyCode::PageDown => app.move_selection(20),
            KeyCode::Home => app.jump_to_top(),
            KeyCode::End => app.jump_to_bottom(),
            KeyCode::Backspace => {
                app.search.mode = SearchMode::Insert;
                app.on_backspace();
            }
            KeyCode::Delete => {
                app.search.mode = SearchMode::Insert;
                app.on_delete();
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.search.sort_by_spread = !app.search.sort_by_spread;
                app.recompute_filter();
                app.status = if app.search.sort_by_spread { "Sorted by spread".into() } else { "Sorted by relevance".into() };
            }
            KeyCode::Char(ch) => {
                app.search.mode = SearchMode::Insert;
                app.on_input(ch);
            }
            KeyCode::Enter => app.enter_detail(),
            _ => {}
        },
    }
    false
}

fn handle_detail_input(app: &mut App, key: event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('b') => app.exit_detail(),
        KeyCode::Char('p') => {
            app.detail.show_percent = !app.detail.show_percent;
            app.status = if app.detail.show_percent { "Chart: % mode".into() } else { "Chart: absolute mode".into() };
        }
        KeyCode::Char('m') => {
            app.detail.show_sma = !app.detail.show_sma;
            app.status = if app.detail.show_sma { "SMA: on".into() } else { "SMA: off".into() };
        }
        KeyCode::Char('g') => {
            app.detail.show_midline = !app.detail.show_midline;
            app.status = if app.detail.show_midline { "Midline: on".into() } else { "Midline: off".into() };
        }
        KeyCode::Char('r') => app.manual_refresh(),
        _ => {}
    }
    false
}
