// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Photosync: Sync photos from camera memory cards to organized directories.
//!
//! This application scans source directories (default: `/media/<username>`) for photos,
//! extracts EXIF metadata, and copies files to `~/Pictures/<camera_dir>/YYYY/MM/DD/`.

use std::io::IsTerminal;
use std::sync::mpsc::{self, Receiver};

use anyhow::{Context, Result};
use clap::Parser;

mod config;
mod pipeline;
mod progress;
mod tui;

use config::{Args, load_config};
use progress::{ProgressMsg, Summary};

/// Channel buffer size for the progress channel.
const PROGRESS_BUFFER_SIZE: usize = 1024;

/// Run the text-mode progress display.
fn run_text_mode(rx: Receiver<ProgressMsg>) -> Summary {
    let mut summary = Summary::default();

    for msg in rx {
        // Update summary statistics (returns true on Done)
        if summary.update(&msg) {
            break;
        }

        // Print progress (text-mode specific)
        match &msg {
            ProgressMsg::ScanningDir(path) => {
                println!("Scanning: {}", path.display());
            }
            ProgressMsg::ScanComplete => {
                println!("Scan complete.");
            }
            ProgressMsg::ExifExtracted { path, model } => {
                println!("Found: {} ({})", path.display(), model);
            }
            ProgressMsg::CopyStarted { src, dest, .. } => {
                println!("Copying: {} -> {}", src.display(), dest.display());
            }
            ProgressMsg::CopyComplete {
                filename,
                size,
                duration,
            } => {
                let speed = progress::format_speed(*size, *duration);
                let bytes = progress::format_bytes(*size);
                println!("  Done: {filename} ({bytes}, {speed})");
            }
            ProgressMsg::CopySkipped { filename } => {
                println!("  Skipped (exists): {filename}");
            }
            ProgressMsg::CopyError { filename, error } => {
                eprintln!("  Error: {filename}: {error}");
            }
            ProgressMsg::SuspiciousDuplicate { src, .. } => {
                eprintln!("  WARNING: Suspicious duplicate: {}", src.display());
            }
            _ => {}
        }
    }

    summary
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config = load_config()?;

    // Determine paths
    let source_dir = config.source_dir(args.source.as_ref())?;
    let target_dir = config.target_dir(args.target.as_ref())?;

    let dry_run = args.dry_run;
    let use_tui = std::io::stdout().is_terminal() && !args.no_tui;

    eprintln!("Source: {}", source_dir.display());
    eprintln!("Target: {}", target_dir.display());

    // Create progress channel for TUI
    // Note: if buffer is full, workers block.
    let (progress_tx, progress_rx) = mpsc::sync_channel::<ProgressMsg>(PROGRESS_BUFFER_SIZE);

    // Spawn the pipeline (walker, processor, handler, and monitor)
    let monitor_handle =
        pipeline::spawn_pipeline(source_dir, target_dir, config, dry_run, progress_tx);

    // Run TUI or text mode
    if use_tui {
        tui::run_tui(progress_rx).context("TUI error")?;
    } else {
        let summary = run_text_mode(progress_rx);
        println!("{summary}");
    }

    // Wait for the monitor thread to finish (which implies all workers are done)
    monitor_handle.join().expect("Monitor thread panicked");
    Ok(())
}
