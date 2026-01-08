// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::{NaiveDate, NaiveDateTime};
use std::sync::mpsc::{Receiver, SyncSender as Sender};
use nom_exif::{Exif, ExifIter, ExifTag, MediaParser, MediaSource};
use walkdir::WalkDir;

use crate::config::Config;
use crate::progress::ProgressMsg;

/// Represents a file with complete EXIF metadata.
#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub model: String,
    pub date: NaiveDate,
}

/// Helper to send progress messages, suppressing errors in release mode (or logging via side channel if needed).
fn send_progress(tx: &Sender<ProgressMsg>, msg: ProgressMsg) {
    let _ = tx.send(msg);
}

/// Helper to report errors to the UI
fn report_error(tx: &Sender<ProgressMsg>, filename: impl Into<String>, error: impl Into<String>) {
    send_progress(
        tx,
        ProgressMsg::CopyError {
            filename: filename.into(),
            error: error.into(),
        },
    );
}

/// Discovers all files under source directories (recursively)
/// and sends their paths through the provided channel.
#[allow(clippy::needless_pass_by_value)]
pub fn file_walker(source_dir: PathBuf, tx: Sender<PathBuf>, progress_tx: Sender<ProgressMsg>) {
    // Recursively walk the provided source directory to find all files.
    // This allows flexible usage: the user can point to a root media folder (e.g., /media/user)
    // or deeper into a specific card structure (e.g., /media/user/EOS_DIGITAL/DCIM).
    // Efficiency note: If the user provides a high-level root with many non-photo files,
    // this will iterate them all. Future optimizations could check for "DCIM" subdirectories.

    send_progress(&progress_tx, ProgressMsg::ScanningDir(source_dir.clone()));

    for walk_entry in WalkDir::new(&source_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if walk_entry.file_type().is_file() {
            let path = walk_entry.into_path();
            send_progress(&progress_tx, ProgressMsg::FileFound);
            if tx.send(path).is_err() {
                return;
            }
        }
    }

    send_progress(&progress_tx, ProgressMsg::ScanComplete);
}

/// Processes file paths received from the channel.
/// Extracts EXIF data and forwards files with complete metadata to the output channel.
#[allow(clippy::needless_pass_by_value)]
pub fn file_processor(
    rx: Receiver<PathBuf>,
    tx: Sender<FileInfo>,
    progress_tx: Sender<ProgressMsg>,
) {
    let mut parser = MediaParser::new();

    for path in rx {
        let Ok(ms) = MediaSource::file_path(&path) else {
            continue;
        };

        if !ms.has_exif() {
            continue;
        }

        let iter: ExifIter = match parser.parse(ms) {
            Ok(iter) => iter,
            Err(_) => continue,
        };

        let exif: Exif = iter.into();

        let model = exif
            .get(ExifTag::Model)
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        // Robust date parsing using chrono
        let date = exif
            .get(ExifTag::DateTimeOriginal)
            .and_then(|v| parse_exif_date(&v.to_string()));

        if let (Some(model), Some(date)) = (model, date) {
            send_progress(
                &progress_tx,
                ProgressMsg::ExifExtracted {
                    path: path.clone(),
                    model: model.clone(),
                },
            );
            let file_info = FileInfo { path, model, date };
            if tx.send(file_info).is_err() {
                return;
            }
        }
    }
}

/// Parses an EXIF date string into a `NaiveDate`.
/// Supports standard EXIF format (YYYY:MM:DD HH:MM:SS) and common variations.
fn parse_exif_date(s: &str) -> Option<NaiveDate> {
    let clean_s = s.trim();

    // Define formats to try
    let formats = [
        "%Y:%m:%d %H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        // Date only formats
        "%Y:%m:%d",
        "%Y-%m-%d",
        "%Y/%m/%d",
    ];

    for fmt in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(clean_s, fmt) {
            return Some(dt.date()); // Convert NaiveDateTime to NaiveDate
        }
        if let Ok(dt) = NaiveDate::parse_from_str(clean_s, fmt) {
            return Some(dt);
        }
    }

    // Fallback parsing for weird separators or non-standard formats
    if s.len() >= 10 {
        let date_part = &s[..10];
        let normalized = date_part.replace([':', '-'], "/");

        // Validation: should be YYYY/MM/DD
        if normalized.chars().filter(|c| *c == '/').count() == 2
            && normalized.chars().all(|c| c.is_ascii_digit() || c == '/')
        {
            // Semantic check and conversion
            let parts: Vec<&str> = normalized.split('/').collect();
            if parts.len() == 3
                && let (Ok(y), Ok(m), Ok(d)) = (
                    parts[0].parse::<i32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<u32>(),
                )
                && let Some(date) = NaiveDate::from_ymd_opt(y, m, d)
            {
                return Some(date);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exif_date_standard() {
        assert_eq!(
            parse_exif_date("2023:10:25 14:30:00"),
            Some(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_exif_date_dashes() {
        assert_eq!(
            parse_exif_date("2023-10-25 14:30:00"),
            Some(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_exif_date_slashes() {
        assert_eq!(
            parse_exif_date("2023/10/25 14:30:00"),
            Some(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_exif_date_date_only() {
        assert_eq!(
            parse_exif_date("2023:10:25"),
            Some(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap())
        );
        assert_eq!(
            parse_exif_date("2023-10-25"),
            Some(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_exif_date_fallback() {
        // Fallback for slightly malformed or unexpected formats that start with the date
        assert_eq!(
            parse_exif_date("2023:10:25 random garbage"),
            Some(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_exif_date_invalid() {
        assert_eq!(parse_exif_date("not a date"), None);
        // "0000:00:00" is strictly invalid now thanks to from_ymd_opt
        assert_eq!(parse_exif_date("0000:00:00 00:00:00"), None);
    }
}

/// Final handler for files with complete metadata.
/// Copies files to `target_dir/<camera_dir>/YYYY/MM/DD/`
#[allow(clippy::needless_pass_by_value)]
pub fn file_handler(
    rx: Receiver<FileInfo>,
    target_dir: PathBuf,
    config: Config,
    dry_run: bool,
    progress_tx: Sender<ProgressMsg>,
) {
    for info in rx {
        let Some(camera_dir) = config.get_dest_dir(&info.model) else {
            // Using a warning message via the UI instead of eprintln
            report_error(
                &progress_tx,
                info.path.display().to_string(),
                format!("Unknown camera model '{}', skipping", info.model),
            );
            continue;
        };

        // Build destination path using platform-agnostic joins
        let dest_dir = target_dir
            .join(camera_dir)
            .join(info.date.format("%Y").to_string())
            .join(info.date.format("%m").to_string())
            .join(info.date.format("%d").to_string());

        let Some(filename) = info.path.file_name() else {
            report_error(
                &progress_tx,
                info.path.display().to_string(),
                "No filename in path",
            );
            continue;
        };
        let filename_str = filename.to_string_lossy().into_owned();
        let dest_path = dest_dir.join(filename);

        if dest_path.exists() {
            send_progress(
                &progress_tx,
                ProgressMsg::CopySkipped {
                    filename: filename_str,
                },
            );
            continue;
        }

        let size = fs::metadata(&info.path).map(|m| m.len()).unwrap_or(0);

        if dry_run {
            send_progress(
                &progress_tx,
                ProgressMsg::CopyStarted {
                    src: info.path.clone(),
                    dest: dest_path.clone(),
                    size,
                },
            );
            send_progress(
                &progress_tx,
                ProgressMsg::CopyComplete {
                    filename: filename_str,
                    size,
                    duration: Duration::from_millis(1),
                },
            );
            continue;
        }

        if let Err(e) = fs::create_dir_all(&dest_dir) {
            report_error(
                &progress_tx,
                filename_str,
                format!("Failed to create directory: {e}"),
            );
            continue;
        }

        send_progress(
            &progress_tx,
            ProgressMsg::CopyStarted {
                src: info.path.clone(),
                dest: dest_path.clone(),
                size,
            },
        );

        let start = Instant::now();
        if let Err(e) = fs::copy(&info.path, &dest_path) {
            report_error(&progress_tx, filename_str, format!("Copy failed: {e}"));
            continue;
        }
        let duration = start.elapsed();

        send_progress(
            &progress_tx,
            ProgressMsg::CopyComplete {
                filename: filename_str,
                size,
                duration,
            },
        );
    }
}
