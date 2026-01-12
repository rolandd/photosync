// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Text-mode progress display for Photosync.
//!
//! Provides a simple streaming output for non-terminal environments,
//! printing each progress event as it occurs.

use std::sync::mpsc::Receiver;

use crate::progress::{self, ProgressMsg, Summary};

/// Run the text-mode progress display.
pub fn run_text_mode(rx: Receiver<ProgressMsg>) -> Summary {
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
            ProgressMsg::ScanError { path, error } => {
                eprintln!("  Scan Error: {}: {}", path.display(), error);
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
