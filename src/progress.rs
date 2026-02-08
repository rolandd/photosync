// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Progress messaging and shared utilities for inter-thread communication.
//!
//! This module defines the [`ProgressMsg`] enum used to communicate progress
//! updates from worker threads to the UI, along with shared formatting utilities.

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use crate::paths::{DestPath, SourcePath};

/// Constants for byte size formatting.
const BYTES_PER_KB: u64 = 1024;
const BYTES_PER_MB: u64 = 1024 * 1024;
const BYTES_PER_GB: u64 = 1024 * 1024 * 1024;

/// Progress messages sent from worker threads to the TUI.
#[derive(Debug, Clone)]
pub enum ProgressMsg {
    // From file_walker
    ScanningDir(SourcePath),
    /// Error accessing a directory or file during scan.
    ScanError {
        path: SourcePath,
        error: String,
    },
    FileFound,
    ScanComplete,

    // From file_processor
    ExifExtracted {
        path: SourcePath,
        model: String,
    },

    // From file_handler
    CopyStarted {
        src: SourcePath,
        dest: DestPath,
        size: u64,
    },
    CopyComplete {
        filename: String,
        size: u64,
        duration: Duration,
    },
    CopySkipped {
        filename: String,
    },
    CopyError {
        filename: String,
        error: String,
    },
    /// File with same name exists at destination but has different contents.
    SuspiciousDuplicate {
        src: SourcePath,
        dest: DestPath,
    },
    /// Camera model not found in configuration (when template needs {camera}).
    UnknownCamera {
        model: String,
    },

    // Sentinel - all workers done
    Done,
}

/// Format a byte count as a human-readable string.
///
/// Returns values like "1.5 GB", "128.3 MB", "4.2 KB", or "512 B".
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= BYTES_PER_GB {
        format!("{:.2} GB", bytes as f64 / BYTES_PER_GB as f64)
    } else if bytes >= BYTES_PER_MB {
        format!("{:.1} MB", bytes as f64 / BYTES_PER_MB as f64)
    } else if bytes >= BYTES_PER_KB {
        format!("{:.1} KB", bytes as f64 / BYTES_PER_KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a transfer speed as a human-readable string.
///
/// Returns values like "125.3 MB/s" or an empty string if duration is zero.
pub fn format_speed(bytes: u64, duration: Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs > 0.0 {
        let bytes_per_sec = bytes as f64 / secs;
        format!("{}/s", format_bytes(bytes_per_sec as u64))
    } else {
        String::new()
    }
}

/// Summary statistics for a sync operation.
#[derive(Debug, Clone, Default)]
pub struct Summary {
    pub files_copied: u64,
    pub files_skipped: u64,
    pub files_errored: u64,
    pub files_found: u64,
    pub bytes_copied: u64,
    pub total_duration: Duration,
    /// Files that matched by name but had different contents.
    pub suspicious_duplicates: Vec<(SourcePath, DestPath)>,
    /// Camera models not found in configuration, with count of files skipped.
    pub unknown_cameras: BTreeMap<String, u32>,
}

impl Summary {
    /// Returns the total number of files processed (copied, skipped, or errored).
    pub fn total_processed(&self) -> u64 {
        self.files_copied
            + self.files_skipped
            + self.files_errored
            + self
                .unknown_cameras
                .values()
                .map(|&c| c as u64)
                .sum::<u64>()
    }

    /// Update summary statistics from a progress message.
    ///
    /// Returns `true` if this was a `Done` message, `false` otherwise.
    pub fn update(&mut self, msg: &ProgressMsg) -> bool {
        match msg {
            ProgressMsg::FileFound => {
                self.files_found += 1;
            }
            ProgressMsg::CopyComplete { size, duration, .. } => {
                self.files_copied += 1;
                self.bytes_copied += *size;
                self.total_duration += *duration;
            }
            ProgressMsg::CopySkipped { .. } => {
                self.files_skipped += 1;
            }
            ProgressMsg::CopyError { .. } => {
                self.files_errored += 1;
            }
            ProgressMsg::SuspiciousDuplicate { src, dest } => {
                self.suspicious_duplicates.push((src.clone(), dest.clone()));
            }
            // Count scan errors as generic errors
            ProgressMsg::ScanError { .. } => {
                self.files_errored += 1;
            }
            ProgressMsg::UnknownCamera { model, .. } => {
                *self.unknown_cameras.entry(model.clone()).or_insert(0) += 1;
            }
            ProgressMsg::Done => return true,
            // ScanningDir, ScanComplete, ExifExtracted, CopyStarted - no summary update
            _ => {}
        }
        false
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let speed_str = if self.files_copied > 0 && self.total_duration.as_secs_f64() > 0.0 {
            let speed = format_speed(self.bytes_copied, self.total_duration);
            format!(", {speed} overall")
        } else {
            String::new()
        };
        let error_str = if self.files_errored > 0 {
            format!(", {} errors", self.files_errored)
        } else {
            String::new()
        };
        write!(
            f,
            "Summary: {} copied, {} skipped (duplicates), {} total files found{}{}",
            self.files_copied, self.files_skipped, self.files_found, error_str, speed_str
        )?;

        // Print suspicious duplicates report
        if !self.suspicious_duplicates.is_empty() {
            writeln!(f)?;
            writeln!(
                f,
                "\nWARNING: {} suspicious duplicate(s) found (same name, different contents):",
                self.suspicious_duplicates.len()
            )?;
            for (src, dest) in &self.suspicious_duplicates {
                writeln!(f, "  Source: {}", src)?;
                writeln!(f, "    Dest: {}", dest)?;
            }
        }

        // Print unknown cameras report
        if !self.unknown_cameras.is_empty() {
            writeln!(f)?;
            let total_skipped: u32 = self.unknown_cameras.values().sum();
            writeln!(
                f,
                "\nWARNING: {} file(s) skipped due to unknown camera model(s):",
                total_skipped
            )?;
            // BTreeMap is already sorted by key
            for (model, count) in &self.unknown_cameras {
                writeln!(f, "  \"{model}\": {count} file(s)")?;
            }
            writeln!(
                f,
                "Run 'photosync --init' to create a config, or edit your config to add mappings."
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::paths::{DestPath, SourcePath};

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_format_bytes_bytes() {
        assert_eq!(format_bytes(512), "512 B");
    }

    #[test]
    fn test_format_bytes_kb() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn test_format_bytes_mb() {
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1536 * 1024), "1.5 MB");
    }

    #[test]
    fn test_format_bytes_gb() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.50 GB");
    }

    #[test]
    fn test_format_speed_zero_duration() {
        assert_eq!(format_speed(1024, Duration::ZERO), "");
    }

    #[test]
    fn test_format_speed_normal() {
        // 10 MB in 1 second = 10 MB/s
        let result = format_speed(10 * 1024 * 1024, Duration::from_secs(1));
        assert_eq!(result, "10.0 MB/s");
    }

    #[test]
    fn test_summary_display() {
        let summary = Summary {
            files_copied: 5,
            files_skipped: 2,
            files_errored: 0,
            files_found: 10,
            bytes_copied: 50 * 1024 * 1024,
            total_duration: Duration::from_secs(5),
            ..Default::default()
        };
        let output = summary.to_string();
        assert!(output.contains("5 copied"));
        assert!(output.contains("2 skipped"));
        assert!(output.contains("10 total files found"));
        assert!(output.contains("10.0 MB/s overall"));
        assert!(!output.contains("error")); // No errors shown when count is 0
    }

    #[test]
    fn test_summary_display_with_errors() {
        let summary = Summary {
            files_copied: 3,
            files_skipped: 1,
            files_errored: 2,
            files_found: 8,
            bytes_copied: 30 * 1024 * 1024,
            total_duration: Duration::from_secs(3),
            ..Default::default()
        };
        let output = summary.to_string();
        assert!(output.contains("3 copied"));
        assert!(output.contains("1 skipped"));
        assert!(output.contains("2 errors"));
        assert!(output.contains("8 total files found"));
    }

    #[test]
    fn test_summary_display_no_copies() {
        let summary = Summary::default();
        let output = summary.to_string();
        assert!(output.contains("0 copied"));
        assert!(!output.contains("overall")); // No speed shown when no copies
        assert!(!output.contains("error")); // No errors shown when count is 0
    }

    #[test]
    fn test_summary_display_with_suspicious_duplicates() {
        let summary = Summary {
            files_copied: 2,
            files_skipped: 3,
            suspicious_duplicates: vec![
                (
                    SourcePath::new(PathBuf::from("/src/IMG_001.jpg")),
                    DestPath::new(PathBuf::from("/dest/IMG_001.jpg")),
                ),
                (
                    SourcePath::new(PathBuf::from("/src/IMG_002.jpg")),
                    DestPath::new(PathBuf::from("/dest/IMG_002.jpg")),
                ),
            ],
            ..Default::default()
        };
        let output = summary.to_string();
        assert!(output.contains("2 suspicious duplicate(s)"));
        assert!(output.contains("same name, different contents"));
        assert!(output.contains("/src/IMG_001.jpg"));
        assert!(output.contains("/dest/IMG_001.jpg"));
        assert!(output.contains("/src/IMG_002.jpg"));
    }

    #[test]
    fn test_summary_total_processed() {
        let mut summary = Summary::default();
        summary.files_copied = 10;
        summary.files_skipped = 5;
        summary.files_errored = 2;
        summary.unknown_cameras.insert("ModelA".to_string(), 3);
        summary.unknown_cameras.insert("ModelB".to_string(), 1);

        // 10 + 5 + 2 + 3 + 1 = 21
        assert_eq!(summary.total_processed(), 21);
    }

    #[test]
    fn test_summary_update() {
        let mut summary = Summary::default();

        // FileFound increments files_found
        assert!(!summary.update(&ProgressMsg::FileFound));
        assert_eq!(summary.files_found, 1);

        // CopyComplete updates counters
        assert!(!summary.update(&ProgressMsg::CopyComplete {
            filename: "test.jpg".to_string(),
            size: 1024,
            duration: Duration::from_millis(100),
        }));
        assert_eq!(summary.files_copied, 1);
        assert_eq!(summary.bytes_copied, 1024);

        // CopySkipped increments skipped
        assert!(!summary.update(&ProgressMsg::CopySkipped {
            filename: "skip.jpg".to_string(),
        }));
        assert_eq!(summary.files_skipped, 1);

        // CopyError increments errored
        assert!(!summary.update(&ProgressMsg::CopyError {
            filename: "err.jpg".to_string(),
            error: "test error".to_string(),
        }));
        assert_eq!(summary.files_errored, 1);

        // SuspiciousDuplicate adds to list
        assert!(!summary.update(&ProgressMsg::SuspiciousDuplicate {
            src: SourcePath::new(PathBuf::from("/src/dup.jpg")),
            dest: DestPath::new(PathBuf::from("/dest/dup.jpg")),
        }));
        assert_eq!(summary.suspicious_duplicates.len(), 1);

        // Done returns true
        assert!(summary.update(&ProgressMsg::Done));
    }
}
