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
/// - Windows reserved filenames (CON, PRN, AUX, NUL, COM1-9, LPT1-9) -> prepends "_"
/// - Trims trailing spaces and dots (Windows requirement)
///
/// If the resulting filename is empty, returns "_".
pub fn sanitize_filename(s: &str) -> String {
    // Optimization: Check if sanitization is needed to avoid unnecessary processing
    // 1. Check for invalid characters
    // 2. Check for trailing dots/spaces
    // 3. Check for empty string
    // 4. Check for Windows reserved names (case-insensitive)
    let is_reserved = is_windows_reserved(s);
    let needs_char_replacement = s.chars().any(|c| {
        c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
    });
    let needs_trimming = s.ends_with([' ', '.']);
    let is_empty = s.is_empty();

    if !needs_char_replacement && !needs_trimming && !is_empty && !is_reserved {
        return s.to_string();
    }

    // 1. Replace invalid characters
    let mut sanitized: String = s
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
    // Optimization: Use truncate instead of creating a new string slice/allocation if possible
    let trim_len = sanitized.trim_end_matches([' ', '.']).len();
    sanitized.truncate(trim_len);

    // 3. Handle Empty or Reserved Filenames
    if sanitized.is_empty() {
        return "_".to_string();
    }

    // Check if the sanitized name became a reserved name
    if is_windows_reserved(&sanitized) {
        sanitized.insert(0, '_');
    }

    sanitized
}

/// Checks if the filename is a Windows reserved device name (CON, PRN, etc.)
fn is_windows_reserved(s: &str) -> bool {
    // Reserved names are case-insensitive on Windows
    let upper = s.to_ascii_uppercase();
    // Check for exact match or match with extension (e.g., CON.txt is also invalid)
    // Actually, on Windows, "CON.txt" is invalid, but checking the stem is usually enough.
    // However, the stem check is complex due to multiple dots.
    // Simple rule: if the filename (without extension) matches a reserved name.

    // Split by dot to get the stem (first part)
    // Note: Windows treats "CON.txt", "CON.foo.bar", "CON" all as the device CON.
    let stem = upper.split('.').next().unwrap_or("");

    matches!(
        stem,
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
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

    #[test]
    fn test_sanitize_filename_reserved_windows() {
        assert_eq!(sanitize_filename("CON"), "_CON");
        assert_eq!(sanitize_filename("prn.txt"), "_prn.txt");
        assert_eq!(sanitize_filename("Aux"), "_Aux");
        assert_eq!(sanitize_filename("NUL"), "_NUL");
        assert_eq!(sanitize_filename("com1"), "_com1");
        assert_eq!(sanitize_filename("LPT9.jpg"), "_LPT9.jpg");
        // Ensure non-reserved names are fine
        assert_eq!(sanitize_filename("CONE"), "CONE");
        assert_eq!(sanitize_filename("auxil"), "auxil");
    }
}
