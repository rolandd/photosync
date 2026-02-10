// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Type-safe path wrappers for source and destination paths.
//!
//! These newtypes provide compile-time distinction between source (input)
//! and destination (output) paths, preventing accidental mix-ups.

use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// Sanitizes a string by replacing control characters with '?' to prevent terminal injection.
///
/// This handles:
/// - ANSI escape codes (start with \x1b)
/// - Newlines, tabs, carriage returns
/// - Other control characters
pub fn sanitize_str(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

/// Sanitizes a filename by replacing control characters and Windows-restricted characters
/// with '_' to ensure cross-platform compatibility and prevent security issues.
///
/// Replaces:
/// - Control characters (0x00-0x1F, 0x7F)
/// - Windows reserved characters: < > : " / \ | ? *
/// - Trims trailing spaces and dots (Windows requirement)
///
/// If the resulting filename is empty, returns "_".
pub fn sanitize_filename(s: &str) -> String {
    // 1. Replace invalid characters
    let sanitized: String = s
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();

    // 2. Trim trailing spaces and dots (Windows disallows these at end of filename)
    let trimmed = sanitized.trim_end_matches(|c| c == ' ' || c == '.');

    if trimmed.is_empty() {
        // Fallback if filename becomes empty (e.g. was just "..." or "   ")
        "_".to_string()
    } else {
        trimmed.to_string()
    }
}

/// A source file path (input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePath(PathBuf);

impl SourcePath {
    /// Creates a new `SourcePath` from a `PathBuf`.
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Deref for SourcePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for SourcePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for SourcePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Sanitize path for display to prevent terminal injection
        write!(f, "{}", sanitize_str(&self.0.to_string_lossy()))
    }
}

impl From<PathBuf> for SourcePath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

/// A destination file path (output).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestPath(PathBuf);

impl DestPath {
    /// Creates a new `DestPath` from a `PathBuf`.
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Deref for DestPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for DestPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for DestPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Sanitize path for display to prevent terminal injection
        write!(f, "{}", sanitize_str(&self.0.to_string_lossy()))
    }
}

impl From<PathBuf> for DestPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_str_normal() {
        assert_eq!(sanitize_str("normal_string"), "normal_string");
        assert_eq!(sanitize_str("path/to/file.txt"), "path/to/file.txt");
    }

    #[test]
    fn test_sanitize_str_control_chars() {
        assert_eq!(sanitize_str("line\nbreak"), "line?break");
        assert_eq!(sanitize_str("tab\tcharacter"), "tab?character");
        assert_eq!(sanitize_str("carriage\rreturn"), "carriage?return");
    }

    #[test]
    fn test_sanitize_str_ansi_escape() {
        // Red color ANSI code: \x1b[31m
        let ansi = "\x1b[31mRed Text";
        // \x1b is replaced by ?
        assert_eq!(sanitize_str(ansi), "?[31mRed Text");
    }

    #[test]
    fn test_source_path_display_sanitization() {
        let raw_path = PathBuf::from("dangerous\npath.jpg");
        let source_path = SourcePath::new(raw_path);
        assert_eq!(source_path.to_string(), "dangerous?path.jpg");
    }

    #[test]
    fn test_dest_path_display_sanitization() {
        let raw_path = PathBuf::from("evil\x1b[2Jpath.jpg");
        let dest_path = DestPath::new(raw_path);
        assert_eq!(dest_path.to_string(), "evil?[2Jpath.jpg");
    }

    #[test]
    fn test_sanitize_filename_basics() {
        assert_eq!(sanitize_filename("normal.jpg"), "normal.jpg");
        assert_eq!(
            sanitize_filename("valid-name_123.txt"),
            "valid-name_123.txt"
        );
    }

    #[test]
    fn test_sanitize_filename_windows_reserved() {
        // < > : " / \ | ? *
        assert_eq!(sanitize_filename("foo<bar>.jpg"), "foo_bar_.jpg");
        assert_eq!(sanitize_filename("foo:bar"), "foo_bar");
        assert_eq!(sanitize_filename("quote\"mark"), "quote_mark");
        assert_eq!(sanitize_filename("slash/backslash\\"), "slash_backslash_");
        assert_eq!(
            sanitize_filename("pipe|question?star*"),
            "pipe_question_star_"
        );
    }

    #[test]
    fn test_sanitize_filename_control_chars() {
        assert_eq!(sanitize_filename("newline\n.jpg"), "newline_.jpg");
        assert_eq!(sanitize_filename("tab\t.txt"), "tab_.txt");
    }

    #[test]
    fn test_sanitize_filename_trailing_dots_spaces() {
        assert_eq!(sanitize_filename("end_space "), "end_space");
        assert_eq!(sanitize_filename("end_dot."), "end_dot");
        assert_eq!(sanitize_filename("both. "), "both");
        assert_eq!(sanitize_filename("..."), "_"); // All dots removed -> empty -> fallback
        assert_eq!(sanitize_filename("   "), "_"); // All spaces removed -> empty -> fallback
    }

    #[test]
    fn test_sanitize_filename_empty() {
        assert_eq!(sanitize_filename(""), "_");
    }
}
