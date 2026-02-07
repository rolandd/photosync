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
}
