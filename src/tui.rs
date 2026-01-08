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
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use std::sync::mpsc::Receiver;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

use crate::progress::{self, ProgressMsg, Summary};

/// Maximum number of recent items to display in the activity log.
const MAX_RECENT_ITEMS: usize = 6;

/// Number of recent copy operations used for speed calculation.
const SPEED_WINDOW_SIZE: usize = 10;

/// Item in the recent activity log.
#[derive(Clone)]
struct RecentItem {
    text: String,
    style: Style,
}

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
    // Scan stage
    scanning_dir: Option<PathBuf>,
    files_found: u64,
    scan_complete: bool,

    // Process stage
    files_with_exif: u64,

    // Copy stage
    current_file: Option<(PathBuf, PathBuf, u64)>, // (src, dest, size)
    files_copied: u64,
    files_to_copy: u64,
    files_skipped: u64,
    bytes_copied: u64,
    total_duration: Duration,
    files_errored: u64,

    // Speed calculation (rolling window)
    recent_copies: VecDeque<(u64, Duration)>, // (bytes, duration)

    // Activity log
    recent_items: VecDeque<RecentItem>,

    // State
    done: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            scanning_dir: None,
            files_found: 0,
            scan_complete: false,
            files_with_exif: 0,
            current_file: None,
            files_copied: 0,
            files_to_copy: 0,
            files_skipped: 0,
            bytes_copied: 0,
            total_duration: Duration::ZERO,
            recent_copies: VecDeque::with_capacity(SPEED_WINDOW_SIZE),
            recent_items: VecDeque::with_capacity(MAX_RECENT_ITEMS),
            done: false,
            files_errored: 0,
        }
    }
}

impl App {
    fn add_recent(&mut self, text: String, style: Style) {
        if self.recent_items.len() >= MAX_RECENT_ITEMS {
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

    /// Build a Summary from current state.
    fn summary(&self) -> Summary {
        Summary {
            files_copied: self.files_copied,
            files_skipped: self.files_skipped,
            files_errored: self.files_errored,
            files_found: self.files_found,
            bytes_copied: self.bytes_copied,
            total_duration: self.total_duration,
        }
    }

    fn handle_message(&mut self, msg: ProgressMsg) {
        match msg {
            ProgressMsg::ScanningDir(path) => {
                self.scanning_dir = Some(path);
            }
            ProgressMsg::FileFound => {
                self.files_found += 1;
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
                self.files_copied += 1;
                self.bytes_copied += size;
                self.total_duration += duration;
                self.current_file = None;

                // Add to rolling window
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
                self.files_skipped += 1;
                self.files_to_copy = self.files_to_copy.saturating_sub(1);
                self.add_recent(
                    format!("⚠ {}  (already exists)", filename),
                    Style::default().fg(Color::Yellow),
                );
            }
            ProgressMsg::CopyError { filename, error } => {
                self.files_errored += 1;
                self.add_recent(
                    format!("✗ {}  {}", filename, error),
                    Style::default().fg(Color::Red),
                );
            }
            ProgressMsg::Done => {
                self.done = true;
            }
        }
    }
}

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Outer block
    let block = Block::default()
        .title(" Photosync ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout: vertical chunks
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2), // Scanning status
            Constraint::Length(3), // Current file
            Constraint::Length(2), // Progress bar
            Constraint::Length(2), // Speed/stats
            Constraint::Min(4),    // Recent items
        ])
        .split(inner);

    // Scanning status
    let scan_text = if app.scan_complete {
        format!(
            "Scan complete.  Files found: {}    With EXIF: {}",
            app.files_found, app.files_with_exif
        )
    } else {
        let dir = app
            .scanning_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "...".to_string());
        format!(
            "Scanning: {}  Files: {}  EXIF: {}",
            dir, app.files_found, app.files_with_exif
        )
    };
    let scan_para = Paragraph::new(scan_text).style(Style::default().fg(Color::White));
    frame.render_widget(scan_para, chunks[0]);

    // Current file
    let current_text = if let Some((src, dest, size)) = &app.current_file {
        let filename = src
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();
        format!(
            "Copying: {} ({})\nTo: {}",
            filename,
            progress::format_bytes(*size),
            dest.display()
        )
    } else if app.done {
        "Done!".to_string()
    } else {
        "Waiting...".to_string()
    };
    let current_para = Paragraph::new(current_text).style(Style::default().fg(Color::White));
    frame.render_widget(current_para, chunks[1]);

    // Progress bar
    let total = app.files_to_copy.max(1);
    let progress_ratio = app.files_copied as f64 / total as f64;
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .ratio(progress_ratio.min(1.0))
        .label(format!("{} / {}", app.files_copied, app.files_to_copy));
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
        progress::format_bytes(app.bytes_copied),
        app.files_skipped
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
}

/// Run the TUI, receiving progress messages until Done.
pub fn run_tui(rx: Receiver<ProgressMsg>) -> Result<()> {
    // Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    stdout()
        .execute(EnterAlternateScreen)
        .context("Failed to enter alternate screen")?;

    // Guard ensures cleanup on panic or early return
    let _guard = TerminalGuard;

    let mut terminal =
        Terminal::new(CrosstermBackend::new(stdout())).context("Failed to create terminal")?;

    let mut app = App::default();
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
            // Wait a moment so user can see completion
            std::thread::sleep(Duration::from_secs(2));
            break;
        }
    }

    // Print summary (terminal already restored by guard)
    // Explicitly drop guard first to restore terminal before printing
    drop(_guard);
    println!("{}", app.summary());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_default() {
        let app = App::default();
        assert_eq!(app.files_found, 0);
        assert_eq!(app.files_copied, 0);
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
        let mut app = App::default();
        for i in 0..10 {
            app.add_recent(format!("Item {i}"), Style::default());
        }
        assert_eq!(app.recent_items.len(), MAX_RECENT_ITEMS);
        // Most recent should be first
        assert!(app.recent_items.front().unwrap().text.contains('9'));
    }

    #[test]
    fn test_handle_message_file_found() {
        let mut app = App::default();
        app.handle_message(ProgressMsg::FileFound);
        assert_eq!(app.files_found, 1);
    }

    #[test]
    fn test_handle_message_copy_complete() {
        let mut app = App::default();
        app.handle_message(ProgressMsg::CopyComplete {
            filename: "test.jpg".to_string(),
            size: 1024,
            duration: Duration::from_millis(100),
        });
        assert_eq!(app.files_copied, 1);
        assert_eq!(app.bytes_copied, 1024);
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
    fn test_summary() {
        let mut app = App::default();
        app.files_copied = 5;
        app.files_skipped = 2;
        app.files_found = 10;
        app.bytes_copied = 1024 * 1024;
        app.total_duration = Duration::from_secs(1);

        let summary = app.summary();
        assert_eq!(summary.files_copied, 5);
        assert_eq!(summary.files_skipped, 2);
        assert_eq!(summary.files_found, 10);
    }
}
