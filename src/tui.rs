// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Terminal User Interface for Photosync.
//!
//! Provides a real-time progress display using ratatui, showing:
//! - Current scanning status and file counts
//! - Active file copy progress
//! - Transfer speed based on a rolling window
//! - Recent activity log with successes, skips, and errors

use std::collections::VecDeque;
use std::io::stdout;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};
use std::sync::mpsc::Receiver;

use crate::paths::{DestPath, SourcePath};
use crate::progress::{self, ProgressMsg, Summary};

/// Number of recent copy operations used for speed calculation.
const SPEED_WINDOW_SIZE: usize = 10;

/// Spinner animation frames (vertical bars).
const SPINNER: &[&str] = &[
    " ", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂",
];

/// Item in the recent activity log.
#[derive(Clone)]
struct RecentItem {
    text: String,
    style: Style,
}

/// Fixed size history buffer for TUI limits.
/// This is large enough to handle most terminal sizes without needing resize math.
const HISTORY_SIZE: usize = 100;

/// RAII guard to ensure terminal is restored on panic or early return.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort cleanup; ignore errors since we may be panicking
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

/// Application state for the TUI.
pub struct App {
    /// Aggregated summary statistics (shared logic with text mode).
    summary: Summary,

    // Scan stage
    scanning_dir: Option<SourcePath>,
    scan_complete: bool,

    // Process stage
    files_with_exif: u64,

    // Copy stage
    current_file: Option<(SourcePath, DestPath, u64)>, // (src, dest, size)
    files_to_copy: u64,

    // Speed calculation (rolling window)
    recent_copies: VecDeque<(u64, Duration)>, // (bytes, duration)

    // Activity log
    recent_items: VecDeque<RecentItem>,
    max_recent_items: usize,

    // State
    done: bool,
    spinner_idx: usize,
}

impl App {
    /// Create a new App with specified max recent items.
    fn with_history_size(size: usize) -> Self {
        Self {
            summary: Summary::default(),
            scanning_dir: None,
            scan_complete: false,
            files_with_exif: 0,
            current_file: None,
            files_to_copy: 0,
            recent_copies: VecDeque::with_capacity(SPEED_WINDOW_SIZE),
            recent_items: VecDeque::with_capacity(size),
            max_recent_items: size,
            done: false,
            spinner_idx: 0,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::with_history_size(HISTORY_SIZE)
    }
}

impl App {
    fn tick(&mut self) {
        self.spinner_idx = (self.spinner_idx + 1) % SPINNER.len();
    }

    fn spinner(&self) -> &'static str {
        SPINNER[self.spinner_idx]
    }

    fn add_recent(&mut self, text: String, style: Style) {
        if self.recent_items.len() >= self.max_recent_items {
            self.recent_items.pop_back();
        }
        self.recent_items.push_front(RecentItem { text, style });
    }

    /// Calculate speed in bytes per second from recent copies.
    fn speed_bytes_per_sec(&self) -> f64 {
        if self.recent_copies.is_empty() {
            return 0.0;
        }

        let (total_bytes, total_duration): (u64, Duration) = self
            .recent_copies
            .iter()
            .fold((0, Duration::ZERO), |(b, d), (bytes, dur)| {
                (b + bytes, d + *dur)
            });

        if total_duration.as_secs_f64() > 0.0 {
            total_bytes as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        }
    }

    fn handle_message(&mut self, msg: ProgressMsg) {
        // Update common summary statistics; sets done flag on Done message
        self.done = self.summary.update(&msg);

        // Handle TUI-specific state
        match msg {
            ProgressMsg::ScanningDir(path) => {
                self.scanning_dir = Some(path);
            }
            ProgressMsg::ScanComplete => {
                self.scan_complete = true;
            }
            ProgressMsg::ExifExtracted { .. } => {
                self.files_with_exif += 1;
                self.files_to_copy = self.files_with_exif;
            }
            ProgressMsg::CopyStarted { src, dest, size } => {
                self.current_file = Some((src, dest, size));
            }
            ProgressMsg::CopyComplete {
                filename,
                size,
                duration,
            } => {
                self.current_file = None;

                // Add to rolling window for speed calculation
                if self.recent_copies.len() >= SPEED_WINDOW_SIZE {
                    self.recent_copies.pop_front();
                }
                self.recent_copies.push_back((size, duration));

                let size_str = progress::format_bytes(size);
                self.add_recent(
                    format!(
                        "✓ {}  {}  {:.1}s",
                        filename,
                        size_str,
                        duration.as_secs_f64()
                    ),
                    Style::default().fg(Color::Green),
                );
            }
            ProgressMsg::CopySkipped { filename } => {
                self.files_to_copy = self.files_to_copy.saturating_sub(1);
                self.add_recent(
                    format!("⚠ {}  (already exists)", filename),
                    Style::default().fg(Color::Yellow),
                );
            }
            ProgressMsg::CopyError { filename, error } => {
                self.add_recent(
                    format!("✗ {}  {}", filename, error),
                    Style::default().fg(Color::Red),
                );
            }
            ProgressMsg::SuspiciousDuplicate { src, .. } => {
                let filename = src
                    .file_name()
                    .map(|n| crate::paths::sanitize_str(&n.to_string_lossy()))
                    .unwrap_or_else(|| src.to_string());
                self.add_recent(
                    format!("⚠ {}  (CONTENTS DIFFER!)", filename),
                    Style::default().fg(Color::LightRed),
                );
            }
            ProgressMsg::UnknownCamera { model } => {
                self.add_recent(
                    format!("⊘ Unknown camera: {}", model),
                    Style::default().fg(Color::Magenta),
                );
            }
            ProgressMsg::ScanError { path, error } => {
                self.add_recent(
                    format!("! Scan Error: {}: {}", path, error),
                    Style::default().fg(Color::Red),
                );
            }
            _ => {}
        }
    }
}

fn get_status_color(app: &App) -> Color {
    if app.done {
        if app.summary.files_errored > 0 {
            Color::Red
        } else if !app.summary.suspicious_duplicates.is_empty()
            || !app.summary.unknown_cameras.is_empty()
        {
            Color::Yellow
        } else {
            Color::Green
        }
    } else {
        Color::Cyan
    }
}

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Outer block
    let mut block = Block::default()
        .title(" Photosync ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(get_status_color(app)));

    if !app.done {
        block = block.title_bottom(
            Line::from(" Press 'q' to quit ")
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::DarkGray)),
        );
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: vertical chunks
    let constraints = if app.done {
        // Add footer when done
        vec![
            Constraint::Length(2), // Scanning status
            Constraint::Length(3), // Current file
            Constraint::Length(2), // Progress bar
            Constraint::Length(2), // Speed/stats
            Constraint::Min(4),    // Recent items
            Constraint::Length(1), // Footer prompt
        ]
    } else {
        vec![
            Constraint::Length(2), // Scanning status
            Constraint::Length(3), // Current file
            Constraint::Length(2), // Progress bar
            Constraint::Length(2), // Speed/stats
            Constraint::Min(4),    // Recent items
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(inner);

    // Scanning status
    let scan_text = if app.scan_complete {
        format!(
            "Scan complete.  Files found: {}    With EXIF: {}",
            app.summary.files_found, app.files_with_exif
        )
    } else {
        let dir = app
            .scanning_dir
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "...".to_string());
        format!(
            "{} Scanning: {}  Files: {}  EXIF: {}",
            app.spinner(),
            dir,
            app.summary.files_found,
            app.files_with_exif
        )
    };
    let scan_para = Paragraph::new(scan_text).style(Style::default().fg(Color::White));
    frame.render_widget(scan_para, chunks[0]);

    // Current file
    let current_text = if let Some((src, dest, size)) = &app.current_file {
        let filename = src
            .file_name()
            .map(|n| crate::paths::sanitize_str(&n.to_string_lossy()))
            .unwrap_or_default();
        format!(
            "Copying: {} ({})\nTo: {}",
            filename,
            progress::format_bytes(*size),
            dest
        )
    } else if app.done {
        "✓ Complete!".to_string()
    } else {
        format!("{} Waiting...", app.spinner())
    };
    let current_style = if app.done {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let current_para = Paragraph::new(current_text).style(current_style);
    frame.render_widget(current_para, chunks[1]);

    // Progress bar
    let processed = app.summary.total_processed();
    let total = app.files_with_exif.max(1);
    let progress_ratio = processed as f64 / total as f64;
    let percentage = (progress_ratio * 100.0).min(100.0);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .ratio(progress_ratio.min(1.0))
        .label(format!("{} / {} ({:.0}%)", processed, total, percentage));
    frame.render_widget(gauge, chunks[2]);

    // Speed and stats
    let speed = app.speed_bytes_per_sec();
    let speed_text = if speed > 0.0 {
        format!("{}/s", progress::format_bytes(speed as u64))
    } else {
        "-".to_string()
    };
    let stats_text = format!(
        "Speed: {}    Copied: {}    Skipped: {} (already exist)",
        speed_text,
        progress::format_bytes(app.summary.bytes_copied),
        app.summary.files_skipped
    );
    let stats_para = Paragraph::new(stats_text).style(Style::default().fg(Color::Gray));
    frame.render_widget(stats_para, chunks[3]);

    // Recent items
    let items: Vec<ListItem> = app
        .recent_items
        .iter()
        .map(|item| ListItem::new(item.text.clone()).style(item.style))
        .collect();
    let recent_list = List::new(items).block(
        Block::default()
            .title("Recent")
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(recent_list, chunks[4]);

    // Footer prompt (only when done)
    if app.done {
        let prompt = Paragraph::new(" Press any key to exit ")
            .style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(prompt, chunks[5]);
    }
}

/// Run the TUI, receiving progress messages until Done.
pub fn run_tui(rx: Receiver<ProgressMsg>) -> Result<Summary> {
    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    stdout()
        .execute(EnterAlternateScreen)
        .context("Failed to enter alternate screen")?;

    // Guard ensures cleanup on panic or early return
    let _guard = TerminalGuard;

    let mut terminal =
        Terminal::new(CrosstermBackend::new(stdout())).context("Failed to create terminal")?;

    // We use a fixed history buffer. Ratatui List will handle scrolling/truncation
    // based on available height.
    let mut app = App::with_history_size(HISTORY_SIZE);
    let tick_rate = Duration::from_millis(100);
    let mut last_draw = Instant::now();

    loop {
        // Check for quit key (q or Ctrl+C)
        if event::poll(Duration::from_millis(10)).context("Failed to poll events")?
            && let Event::Key(key) = event::read().context("Failed to read event")?
            && (key.code == KeyCode::Char('q')
                || (key.code == KeyCode::Char('c')
                    && key.modifiers.contains(event::KeyModifiers::CONTROL)))
        {
            break;
        }

        // Process all available messages
        while let Ok(msg) = rx.try_recv() {
            app.handle_message(msg);
        }

        // Redraw at tick rate
        if last_draw.elapsed() >= tick_rate {
            app.tick();
            terminal
                .draw(|f| ui(f, &app))
                .context("Failed to draw frame")?;
            last_draw = Instant::now();
        }

        // Exit when done
        if app.done {
            // Final draw
            terminal
                .draw(|f| ui(f, &app))
                .context("Failed to draw final frame")?;
            // Wait for key press
            loop {
                if event::poll(Duration::from_millis(100)).context("Failed to poll events")?
                    && let Event::Key(_) = event::read().context("Failed to read event")?
                {
                    break;
                }
            }
            break;
        }
    }

    // Explicitly drop guard to restore terminal before returning
    drop(_guard);

    Ok(app.summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use std::path::PathBuf;

    #[test]
    fn test_app_default() {
        let app = App::default();
        assert_eq!(app.summary.files_found, 0);
        assert_eq!(app.summary.files_copied, 0);
        assert!(!app.done);
    }

    #[test]
    fn test_speed_bytes_per_sec_empty() {
        let app = App::default();
        assert_eq!(app.speed_bytes_per_sec(), 0.0);
    }

    #[test]
    fn test_speed_bytes_per_sec_single() {
        let mut app = App::default();
        // 10 MB in 1 second = 10 MB/s = 10485760 bytes/s
        app.recent_copies
            .push_back((10 * 1024 * 1024, Duration::from_secs(1)));
        let speed = app.speed_bytes_per_sec();
        assert!((speed - 10485760.0).abs() < 1.0);
    }

    #[test]
    fn test_speed_bytes_per_sec_multiple() {
        let mut app = App::default();
        // 10 MB in 1 second + 20 MB in 2 seconds = 30 MB in 3 seconds = 10 MB/s
        app.recent_copies
            .push_back((10 * 1024 * 1024, Duration::from_secs(1)));
        app.recent_copies
            .push_back((20 * 1024 * 1024, Duration::from_secs(2)));
        let speed = app.speed_bytes_per_sec();
        assert!((speed - 10485760.0).abs() < 1.0);
    }

    #[test]
    fn test_add_recent_respects_limit() {
        let mut app = App::with_history_size(6);
        for i in 0..10 {
            app.add_recent(format!("Item {i}"), Style::default());
        }
        assert_eq!(app.recent_items.len(), app.max_recent_items);
        // Most recent should be first
        assert!(app.recent_items.front().unwrap().text.contains('9'));
    }

    #[test]
    fn test_handle_message_file_found() {
        let mut app = App::default();
        app.handle_message(ProgressMsg::FileFound);
        assert_eq!(app.summary.files_found, 1);
    }

    #[test]
    fn test_handle_message_copy_complete() {
        let mut app = App::default();
        app.handle_message(ProgressMsg::CopyComplete {
            filename: "test.jpg".to_string(),
            size: 1024,
            duration: Duration::from_millis(100),
        });
        assert_eq!(app.summary.files_copied, 1);
        assert_eq!(app.summary.bytes_copied, 1024);
        assert_eq!(app.recent_copies.len(), 1);
    }

    #[test]
    fn test_handle_message_done() {
        let mut app = App::default();
        assert!(!app.done);
        app.handle_message(ProgressMsg::Done);
        assert!(app.done);
    }

    #[test]
    fn test_handle_message_copy_skipped() {
        let mut app = App::default();
        app.files_to_copy = 5;
        app.handle_message(ProgressMsg::CopySkipped {
            filename: "test.jpg".to_string(),
        });
        assert_eq!(app.summary.files_skipped, 1);
        assert_eq!(app.files_to_copy, 4); // decremented
        assert_eq!(app.recent_items.len(), 1);
        assert!(
            app.recent_items
                .front()
                .unwrap()
                .text
                .contains("already exists")
        );
    }

    #[test]
    fn test_ui_quit_hint() {
        let app = App::default();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &app)).unwrap();

        let buffer = terminal.backend().buffer();
        // Check that the quit hint is present in the buffer at the bottom row (index 23)
        let y = 23;
        let mut row_text = String::new();
        for x in 0..80 {
            row_text.push_str(buffer[(x, y)].symbol());
        }

        assert!(row_text.contains("Press 'q' to quit"));
    }

    #[test]
    fn test_app_tick() {
        let mut app = App::default();
        let initial = app.spinner_idx;
        app.tick();
        assert_eq!(app.spinner_idx, (initial + 1) % SPINNER.len());
    }

    #[test]
    fn test_app_spinner_helper() {
        let mut app = App::default();
        // Starts at 0
        assert_eq!(app.spinner(), SPINNER[0]);
        app.tick();
        assert_eq!(app.spinner(), SPINNER[1]);
    }

    #[test]
    fn test_ui_spinner() {
        let mut app = App::default();
        // Waiting state (current_file is None, done is false)
        // spinner_idx is 0 by default, so SPINNER[0] is " "

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| ui(f, &app)).unwrap();

        let buffer = terminal.backend().buffer();
        // Check for "Waiting..." string
        let mut found_waiting = false;
        for y in 0..24 {
            let mut row_text = String::new();
            for x in 0..80 {
                row_text.push_str(buffer[(x, y)].symbol());
            }
            if row_text.contains("Waiting...") {
                found_waiting = true;
            }
        }
        assert!(found_waiting);

        // Advance tick to get a visible character
        app.tick(); // index 1 -> "▂"
        terminal.draw(|f| ui(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();

        let mut found_spinner = false;
        for y in 0..24 {
            let mut row_text = String::new();
            for x in 0..80 {
                row_text.push_str(buffer[(x, y)].symbol());
            }
            if row_text.contains("▂ Waiting...") {
                found_spinner = true;
                break;
            }
        }
        assert!(found_spinner, "Spinner character not found in UI output");
    }

    #[test]
    fn test_ui_border_color_status() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Success state
        let mut app = App::default();
        app.done = true;
        terminal.draw(|f| ui(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        // Check top-left corner (0,0) style. Border is usually Cyan, should be Green.
        let cell = &buffer[(0, 0)];
        assert_eq!(cell.style().fg, Some(Color::Green));

        // 2. Error state
        let mut app = App::default();
        app.done = true;
        app.summary.files_errored = 1;
        terminal.draw(|f| ui(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let cell = &buffer[(0, 0)];
        assert_eq!(cell.style().fg, Some(Color::Red));

        // 3. Warning state (suspicious duplicate)
        let mut app = App::default();
        app.done = true;
        app.summary.suspicious_duplicates.push((
            SourcePath::new(PathBuf::from("a")),
            DestPath::new(PathBuf::from("b")),
        ));
        terminal.draw(|f| ui(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let cell = &buffer[(0, 0)];
        assert_eq!(cell.style().fg, Some(Color::Yellow));

        // 4. Running state
        let mut app = App::default();
        app.done = false;
        terminal.draw(|f| ui(f, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let cell = &buffer[(0, 0)];
        assert_eq!(cell.style().fg, Some(Color::Cyan));
    }
}
