use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table},
    Terminal,
};

use crate::cli::{App, Args, NodeModule};

fn from_bytes(bytes: u64, in_gb: bool) -> f32 {
    let shift = if in_gb { 30 } else { 20 };
    bytes as f32 / (1 << shift) as f32
}

#[inline]
fn format_duration(seconds: i64) -> String {
    match seconds {
        s if s < 60 => format!("{}s", s),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86400 => format!("{}h", s / 3600),
        s if s < 2592000 => format!("{}d", s / 86400),
        s => format!("{}d", s / 86400),
    }
}

#[inline]
#[allow(dead_code)]
fn get_age(seconds: i64) -> i64 {
    match seconds {
        s if s < 60 => s,
        s if s < 3600 => s / 60,
        s if s < 86400 => s / 3600,
        s if s < 2592000 => s / 86400,
        s => s / 86400,
    }
}

pub fn run_tui(
    modules: Vec<NodeModule>,
    args: Args,
    start: std::time::Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    let mut app = App::new(modules, start);
    let total_size_bytes = app.modules.iter().map(|m| m.size).sum();
    let total_size = from_bytes(total_size_bytes, true);
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(f.area());
            let size_metric = if args.in_gb { "GB" } else { "MB" };
            let header = Table::new(
                vec![Row::new(vec![
                    format!("Total Size: {:.2}GB", total_size),
                    format!("Modules: {}", app.modules.len()),
                    format!("Scan Time: {:?}", app.scan_time),
                    format!(
                        "Total Deleted: {:.2}GB",
                        from_bytes(app.total_deleted, true)
                    ),
                ])],
                &[
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ],
            )
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            if app.modules.is_empty() {
                let message = Paragraph::new("No directories found")
                    .block(Block::default().title("Directories").borders(Borders::ALL))
                    .alignment(Alignment::Center);
                f.render_widget(message, chunks[1]);
            } else {
                let items: Vec<ListItem> = app
                    .modules
                    .iter_mut()
                    .map(|m| {
                        if args.delete_all && !m.deleted {
                            m.delete();
                            app.total_deleted += m.size;
                        }
                        let style = if m.deleted {
                            Style::default().fg(Color::Red)
                        } else if m.is_dangerous {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        };
                        ListItem::new(format!(
                            "{} {} | {} | {:.2}{size_metric} ",
                            if m.deleted { "[deleted]" } else { "" },
                            m.path.display(),
                            format_duration(m.modified),
                            from_bytes(m.size, args.in_gb),
                        ))
                        .style(style)
                    })
                    .collect();
                let modules_list = List::new(items)
                    .block(Block::default().title("Node Modules").borders(Borders::ALL))
                    .highlight_symbol("> ");
                f.render_stateful_widget(
                    modules_list,
                    chunks[1],
                    &mut ListState::default().with_selected(Some(app.scroll)),
                );
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                break;
            }
            app.on_key(key.code);
        }
    }
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub async fn display_spinner(scanning: Arc<AtomicBool>) -> std::io::Result<()> {
    let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut i = 0;

    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    while scanning.load(Ordering::Relaxed) {
        terminal.draw(|f| {
            let text = format!("{} Scanning directories...", spinner[i]);
            let paragraph = Paragraph::new(text).alignment(Alignment::Center);
            f.render_widget(paragraph, f.area());
        })?;

        i = (i + 1) % spinner.len();
        tokio::time::sleep(Duration::from_millis(80)).await;
    }

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub fn confirm_delete_all(target: &str) -> Result<bool, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let confirmed = loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Length(3), Constraint::Length(3)])
                .split(f.area());

            let warning = Paragraph::new(format!(
                "⚠️  WARNING: You are about to delete ALL {target} directories!"
            ))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

            let prompt = Paragraph::new("Press 'y' to confirm or any other key to cancel")
                .alignment(Alignment::Center);

            f.render_widget(warning, chunks[0]);
            f.render_widget(prompt, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('y') => break true,
                _ => break false,
            }
        }
    };

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(confirmed)
}
