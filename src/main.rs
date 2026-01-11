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
mod paths;
mod pipeline;
mod progress;
mod tui;

use config::{Args, config_path, generate_config_template, load_config, validate_template};
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
            // UnknownCamera is tracked in summary, no need to log each instance
            _ => {}
        }
    }

    summary
}

/// Handle --init: create a starter config file.
fn handle_init() -> Result<()> {
    use std::fs;

    let config_path = config_path().context("Could not determine config directory")?;

    if config_path.exists() {
        anyhow::bail!(
            "Config file already exists at: {}\nEdit it directly or delete it first.",
            config_path.display()
        );
    }

    // Create parent directory if needed
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let template = generate_config_template();
    fs::write(&config_path, template)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    println!("Created config file: {}", config_path.display());
    println!("Edit this file to add your camera mappings.");
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --init first (before loading config)
    if args.init {
        return handle_init();
    }

    let mut config = load_config()?;

    // Validate and set CLI template override (uses same validation as config file)
    if let Some(t) = args.template {
        config.dest_template = validate_template(t);
    }

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
