// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Progress messaging and shared utilities for inter-thread communication.
//!
//! This module defines the [`ProgressMsg`] enum used to communicate progress
//! updates from worker threads to the UI, along with shared formatting utilities.

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// Constants for byte size formatting.
const BYTES_PER_KB: u64 = 1024;
const BYTES_PER_MB: u64 = 1024 * 1024;
const BYTES_PER_GB: u64 = 1024 * 1024 * 1024;

/// Progress messages sent from worker threads to the TUI.
#[derive(Debug, Clone)]
pub enum ProgressMsg {
    // From file_walker
    ScanningDir(PathBuf),
    FileFound,
    ScanComplete,

    // From file_processor
    ExifExtracted {
        path: PathBuf,
        model: String,
    },

    // From file_handler
    CopyStarted {
        src: PathBuf,
        dest: PathBuf,
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
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
