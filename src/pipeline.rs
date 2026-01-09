use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{NaiveDate, NaiveDateTime};
use nom_exif::{Exif, ExifIter, ExifTag, MediaParser, MediaSource};
use walkdir::WalkDir;

use crate::config::Config;
use crate::progress::ProgressMsg;

/// Channel buffer size for pipeline stages.
const CHANNEL_BUFFER_SIZE: usize = 1024;

/// Represents a file with complete EXIF metadata.
#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub model: String,
    pub date: NaiveDate,
}

/// Spawns the processing pipeline threads.
///
/// This function sets up the channels and spawns the worker threads:
/// 1. Walker: Scans the source directory.
/// 2. Processor: Extracts EXIF data.
/// 3. Handler: Copies files to the destination.
/// 4. Monitor: Waits for all workers to finish and sends the Done signal.
///
/// Returns the `JoinHandle` of the monitor thread.
pub fn spawn_pipeline(
    source_dir: PathBuf,
    target_dir: PathBuf,
    config: Config,
    dry_run: bool,
    progress_tx: SyncSender<ProgressMsg>,
) -> thread::JoinHandle<()> {
    // Create pipeline channels
    let (walker_tx, processor_rx) = mpsc::sync_channel::<PathBuf>(CHANNEL_BUFFER_SIZE);
    let (processor_tx, handler_rx) = mpsc::sync_channel::<FileInfo>(CHANNEL_BUFFER_SIZE);

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
    thread::spawn(move || {
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
    })
}

/// Helper to send progress messages, suppressing errors.
fn send_progress(tx: &SyncSender<ProgressMsg>, msg: ProgressMsg) {
    let _ = tx.send(msg);
}

/// Helper to report errors to the UI
fn report_error(
    tx: &SyncSender<ProgressMsg>,
    filename: impl Into<String>,
    error: impl Into<String>,
) {
    send_progress(
        tx,
        ProgressMsg::CopyError {
            filename: filename.into(),
            error: error.into(),
        },
    );
}

/// Chunk size for file comparison (128 KB).
const FILE_COMPARE_CHUNK_SIZE: usize = 128 * 1024;

/// Compare two files for equality.
fn files_are_equal(path1: &Path, path2: &Path) -> io::Result<bool> {
    // Fast path: compare sizes first
    let meta1 = fs::metadata(path1)?;
    let meta2 = fs::metadata(path2)?;
    if meta1.len() != meta2.len() {
        return Ok(false);
    }

    // Chunked byte comparison
    let mut reader1 = BufReader::with_capacity(FILE_COMPARE_CHUNK_SIZE, File::open(path1)?);
    let mut reader2 = BufReader::with_capacity(FILE_COMPARE_CHUNK_SIZE, File::open(path2)?);

    let mut buf1 = vec![0u8; FILE_COMPARE_CHUNK_SIZE];
    let mut buf2 = vec![0u8; FILE_COMPARE_CHUNK_SIZE];

    loop {
        let n1 = reader1.read(&mut buf1)?;
        let n2 = reader2.read(&mut buf2)?;

        if n1 != n2 || buf1[..n1] != buf2[..n2] {
            return Ok(false);
        }

        if n1 == 0 {
            return Ok(true);
        }
    }
}

/// Discovers all files under source directories.
fn file_walker(source_dir: PathBuf, tx: SyncSender<PathBuf>, progress_tx: SyncSender<ProgressMsg>) {
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
fn file_processor(
    rx: Receiver<PathBuf>,
    tx: SyncSender<FileInfo>,
    progress_tx: SyncSender<ProgressMsg>,
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
            return Some(dt.date());
        }
        if let Ok(dt) = NaiveDate::parse_from_str(clean_s, fmt) {
            return Some(dt);
        }
    }

    // Fallback parsing for weird separators
    if s.len() >= 10 {
        let date_part = &s[..10];
        let normalized = date_part.replace([':', '-'], "/");

        if normalized.chars().filter(|c| *c == '/').count() == 2
            && normalized.chars().all(|c| c.is_ascii_digit() || c == '/')
        {
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

/// Helper to compute the destination directory for a file.
/// Returns `None` if the camera model is unknown.
fn compute_dest_dir(
    target_dir: &Path,
    config: &Config,
    model: &str,
    date: NaiveDate,
) -> Option<PathBuf> {
    let camera_dir = config.get_dest_dir(model)?;
    Some(
        target_dir
            .join(camera_dir)
            .join(date.format("%Y").to_string())
            .join(date.format("%m").to_string())
            .join(date.format("%d").to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;

    /// Helper to create a test file with given content.
    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::File::create(&path)
            .unwrap()
            .write_all(content)
            .unwrap();
        path
    }

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
        assert_eq!(
            parse_exif_date("2023:10:25 random garbage"),
            Some(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap())
        );
    }

    #[test]
    fn test_parse_exif_date_invalid() {
        assert_eq!(parse_exif_date("not a date"), None);
        assert_eq!(parse_exif_date("0000:00:00 00:00:00"), None);
    }

    #[test]
    fn test_files_are_equal_identical() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "file1.txt", b"Hello, world!");
        let file2 = create_test_file(dir.path(), "file2.txt", b"Hello, world!");

        assert!(files_are_equal(&file1, &file2).unwrap());
    }

    #[test]
    fn test_files_are_equal_different_size() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "file1.txt", b"short");
        let file2 = create_test_file(dir.path(), "file2.txt", b"much longer content");

        assert!(!files_are_equal(&file1, &file2).unwrap());
    }

    #[test]
    fn test_files_are_equal_same_size_different_content() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "file1.txt", b"AAAA");
        let file2 = create_test_file(dir.path(), "file2.txt", b"BBBB");

        assert!(!files_are_equal(&file1, &file2).unwrap());
    }

    #[test]
    fn test_files_are_equal_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "exists.txt", b"");
        let file2 = dir.path().join("missing.txt");

        assert!(files_are_equal(&file1, &file2).is_err());
    }
}

/// Final handler for files with complete metadata.
/// Copies files to `target_dir/<camera_dir>/YYYY/MM/DD/`
fn file_handler(
    rx: Receiver<FileInfo>,
    target_dir: PathBuf,
    config: Config,
    dry_run: bool,
    progress_tx: SyncSender<ProgressMsg>,
) {
    for info in rx {
        let Some(dest_dir) = compute_dest_dir(&target_dir, &config, &info.model, info.date) else {
            // Using a warning message via the UI instead of eprintln
            report_error(
                &progress_tx,
                info.path.display().to_string(),
                format!("Unknown camera model '{}', skipping", info.model),
            );
            continue;
        };

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
            // Check if contents are the same
            let is_same = files_are_equal(&info.path, &dest_path).unwrap_or_else(|e| {
                // Log error and assume different (safer)
                report_error(
                    &progress_tx,
                    filename_str.clone(),
                    format!("Error comparing files: {e}"),
                );
                false
            });

            if !is_same {
                // Suspicious: same name but different contents
                send_progress(
                    &progress_tx,
                    ProgressMsg::SuspiciousDuplicate {
                        src: info.path.clone(),
                        dest: dest_path.clone(),
                    },
                );
            }

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
