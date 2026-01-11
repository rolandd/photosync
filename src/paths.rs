// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

//! Type-safe path wrappers for source and destination paths.
//!
//! These newtypes provide compile-time distinction between source (input)
//! and destination (output) paths, preventing accidental mix-ups.

use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};

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
        self.0.display().fmt(f)
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
        self.0.display().fmt(f)
    }
}

impl From<PathBuf> for DestPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}
