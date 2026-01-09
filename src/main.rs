// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Photosync: Sync photos from camera memory cards to organized directories.
//!
//! This application scans source directories (default: `/media/<username>`) for photos,
//! extracts EXIF metadata, and copies files to `~/Pictures/<camera_dir>/YYYY/MM/DD/`.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::thread;

use anyhow::{Context, Result};
use clap::Parser;
use std::sync::mpsc::{self, Receiver};

mod config;
mod pipeline;
mod progress;
mod tui;

use config::{Args, load_config};
use pipeline::{FileInfo, file_handler, file_processor, file_walker};
use progress::{ProgressMsg, Summary};

/// Channel buffer size for pipeline stages.
/// Large enough to allow burst handling while bounding memory.
const CHANNEL_BUFFER_SIZE: usize = 1024;

/// Run the text-mode progress display.
fn run_text_mode(rx: Receiver<ProgressMsg>) -> Summary {
    let mut summary = Summary::default();

    for msg in rx {
        match msg {
            ProgressMsg::ScanningDir(path) => {
                println!("Scanning: {}", path.display());
            }
            ProgressMsg::FileFound => {
                summary.files_found += 1;
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
                summary.files_copied += 1;
                summary.bytes_copied += size;
                summary.total_duration += duration;
                let speed = progress::format_speed(size, duration);
                let bytes = progress::format_bytes(size);
                println!("  Done: {filename} ({bytes}, {speed})");
            }
            ProgressMsg::CopySkipped { filename } => {
                summary.files_skipped += 1;
                println!("  Skipped (exists): {filename}");
            }
            ProgressMsg::CopyError { filename, error } => {
                summary.files_errored += 1;
                eprintln!("  Error: {filename}: {error}");
            }
            ProgressMsg::Done => {
                break;
            }
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

    // Create pipeline channels
    let (walker_tx, processor_rx) = mpsc::sync_channel::<PathBuf>(CHANNEL_BUFFER_SIZE);
    let (processor_tx, handler_rx) = mpsc::sync_channel::<FileInfo>(CHANNEL_BUFFER_SIZE);

    // Create progress channel for TUI
    // Note: if buffer is full, workers block.
    let (progress_tx, progress_rx) = mpsc::sync_channel::<ProgressMsg>(CHANNEL_BUFFER_SIZE);

    // Clone senders for each worker
    let walker_progress = progress_tx.clone();
    let processor_progress = progress_tx.clone();
    let handler_progress = progress_tx.clone();

    // Spawn the producer thread (file walker)
    let walker_handle = thread::spawn(move || {
        file_walker(source_dir, walker_tx, walker_progress);
    });

    // Spawn the processor thread (EXIF extraction)
    let processor_handle = thread::spawn(move || {
        file_processor(processor_rx, processor_tx, processor_progress);
    });

    // Spawn the handler thread (final processing)
    // We clone config for the handler since it needs to look up camera dirs
    let handler_config = config.clone();
    let handler_handle = thread::spawn(move || {
        file_handler(
            handler_rx,
            target_dir,
            handler_config,
            dry_run,
            handler_progress,
        );
    });

    // Spawn a thread to send Done after all workers finish
    let done_tx = progress_tx;
    let done_handle = thread::spawn(move || {
        // Collect thread handles and their names for error reporting
        let handles = [
            (walker_handle, "Walker"),
            (processor_handle, "Processor"),
            (handler_handle, "Handler"),
        ];

        for (handle, name) in handles {
            if let Err(e) = handle.join() {
                eprintln!("{name} thread panicked: {e:?}");
            }
        }

        // Send Done (ignoring send errors if receiver is gone)
        let _ = done_tx.send(ProgressMsg::Done);
    });

    // Run TUI or text mode
    if use_tui {
        tui::run_tui(progress_rx).context("TUI error")?;
    } else {
        let summary = run_text_mode(progress_rx);
        println!("{summary}");
    }

    done_handle.join().expect("Done thread panicked");
    Ok(())
}
