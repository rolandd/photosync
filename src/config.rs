// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;

/// Sync photos from camera memory cards to organized directories.
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Dry run: print what would be done without actually copying files
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Disable TUI and use text output
    #[arg(long)]
    pub no_tui: bool,

    /// Override the source directory (defaults to `/media/$USER` or configured value)
    #[arg(long)]
    pub source: Option<PathBuf>,

    /// Override the target directory (defaults to `$HOME/Pictures` or configured value)
    #[arg(long)]
    pub target: Option<PathBuf>,

    /// Destination directory structure template (default: "{camera}/{year}/{month}/{day}")
    #[arg(long)]
    pub template: Option<String>,
}

/// Configuration file structure.
#[derive(Debug, Default, Clone)]
pub struct Config {
    /// Maps camera model substrings to destination directory names.
    /// Sorted by key length (longest first) for deterministic matching.
    pub camera_dirs: Vec<(String, String)>,

    /// Optional source directory override.
    pub source_dir: Option<PathBuf>,

    /// Optional target directory override.
    pub target_dir: Option<PathBuf>,

    /// Destination directory structure template.
    pub dest_template: Option<String>,
}

/// Raw directories section from TOML.
#[derive(Debug, Deserialize, Default)]
struct RawDirs {
    source: Option<PathBuf>,
    target: Option<PathBuf>,
    template: Option<String>,
}

/// Raw configuration as deserialized from TOML.
#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    dirs: RawDirs,
    #[serde(default)]
    cameras: std::collections::HashMap<String, String>,
}

/// Sorts camera dirs by key length (longest first), then lexicographically.
fn sort_camera_dirs(dirs: &mut [(String, String)]) {
    dirs.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
}

/// Loads the effective configuration.
///
/// Priority:
/// 1. Config file (if present)
/// 2. Defaults
///
/// # Errors
/// Returns an error if a config file exists but fails to parse.
pub fn load_config() -> Result<Config> {
    let config_paths: [Option<PathBuf>; 2] = [
        // Check current directory first
        Some(PathBuf::from("photosync.toml")),
        // Then check user config directory
        dirs::config_dir().map(|p| p.join("photosync/photosync.toml")),
    ];

    for path in config_paths.into_iter().flatten() {
        if let Ok(contents) = fs::read_to_string(&path) {
            let config = parse_config_str(&contents).with_context(|| {
                format!(
                    "Failed to parse config file '{}'.\n\
                     Please check the TOML syntax and ensure all fields are valid.",
                    path.display()
                )
            })?;
            eprintln!("Loaded config from: {}", path.display());
            return Ok(config);
        }
    }

    // No config file found, use defaults
    parse_config_str("")
}

/// Parse a TOML config string into a Config.
/// Missing sections use defaults.
fn parse_config_str(contents: &str) -> Result<Config> {
    let raw: RawConfig = toml::from_str(contents)?;

    // Convert HashMap to sorted Vec, using defaults if empty
    let mut camera_dirs: Vec<_> = if raw.cameras.is_empty() {
        default_camera_dirs()
    } else {
        raw.cameras.into_iter().collect()
    };
    sort_camera_dirs(&mut camera_dirs);

    Ok(Config {
        camera_dirs,
        source_dir: raw.dirs.source,
        target_dir: raw.dirs.target,
        dest_template: raw.dirs.template,
    })
}

/// Default camera model mappings.
fn default_camera_dirs() -> Vec<(String, String)> {
    vec![
        ("EOS R6".into(), "CanonR6-images".into()),
        ("6D".into(), "Canon6D-images".into()),
        ("Hero13".into(), "GoPro-Hero13".into()),
        ("HERO13".into(), "GoPro-Hero13".into()),
        ("Hero8".into(), "GoPro-Hero8".into()),
        ("HERO8".into(), "GoPro-Hero8".into()),
    ]
}

impl Config {
    /// Creates a Config with default camera directories, properly sorted.
    #[cfg(test)]
    fn with_defaults() -> Self {
        let mut camera_dirs = default_camera_dirs();
        sort_camera_dirs(&mut camera_dirs);
        Self {
            camera_dirs,
            source_dir: None,
            target_dir: None,
            dest_template: None,
        }
    }

    /// Maps a camera model string to a destination directory name.
    ///
    /// Uses longest-match-first semantics to ensure deterministic behavior
    /// when a model contains multiple matching substrings.
    pub fn get_dest_dir(&self, camera_model: &str) -> Option<&str> {
        self.camera_dirs
            .iter()
            .find(|(substring, _)| camera_model.contains(substring.as_str()))
            .map(|(_, dest_dir)| dest_dir.as_str())
    }

    /// Resolve the effective source directory.
    /// Priority: CLI arg > Config file > Default (/media/$USER)
    pub fn source_dir(&self, cli_override: Option<&PathBuf>) -> Result<PathBuf> {
        Self::resolve_dir(cli_override, &self.source_dir, || {
            let user = env::var("USER").context("USER environment variable not set")?;
            Ok(PathBuf::from("/media").join(user))
        })
    }

    /// Resolve the effective target directory.
    /// Priority: CLI arg > Config file > Default ($HOME/Pictures)
    pub fn target_dir(&self, cli_override: Option<&PathBuf>) -> Result<PathBuf> {
        Self::resolve_dir(cli_override, &self.target_dir, || {
            dirs::picture_dir().context("Could not determine user Pictures directory")
        })
    }

    /// Helper: resolve a directory with CLI > config > default priority.
    fn resolve_dir(
        cli_override: Option<&PathBuf>,
        config_value: &Option<PathBuf>,
        default: impl FnOnce() -> Result<PathBuf>,
    ) -> Result<PathBuf> {
        if let Some(path) = cli_override {
            return Ok(path.clone());
        }
        if let Some(path) = config_value {
            return Ok(path.clone());
        }
        default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dest_dir_eos_r6() {
        let config = Config::with_defaults();
        assert_eq!(config.get_dest_dir("Canon EOS R6"), Some("CanonR6-images"));
    }

    #[test]
    fn test_get_dest_dir_6d() {
        let config = Config::with_defaults();
        assert_eq!(config.get_dest_dir("Canon EOS 6D"), Some("Canon6D-images"));
    }

    #[test]
    fn test_get_dest_dir_gopro_hero13() {
        let config = Config::with_defaults();
        assert_eq!(
            config.get_dest_dir("GoPro Hero13 Black"),
            Some("GoPro-Hero13")
        );
        assert_eq!(
            config.get_dest_dir("GoPro HERO13 Black"),
            Some("GoPro-Hero13")
        );
    }

    #[test]
    fn test_get_dest_dir_gopro_hero8() {
        let config = Config::with_defaults();
        assert_eq!(
            config.get_dest_dir("GoPro Hero8 Black"),
            Some("GoPro-Hero8")
        );
        assert_eq!(
            config.get_dest_dir("GoPro HERO8 Black"),
            Some("GoPro-Hero8")
        );
    }

    #[test]
    fn test_get_dest_dir_unknown() {
        let config = Config::with_defaults();
        assert_eq!(config.get_dest_dir("Sony A7 III"), None);
        assert_eq!(config.get_dest_dir("Unknown Camera"), None);
    }

    #[test]
    fn test_default_camera_dirs_not_empty() {
        let dirs = default_camera_dirs();
        assert!(!dirs.is_empty());
        assert!(dirs.iter().any(|(k, _)| k == "EOS R6"));
        assert!(dirs.iter().any(|(k, _)| k == "6D"));
    }

    // TOML parsing tests

    #[test]
    fn test_parse_empty_config() {
        let config = parse_config_str("").unwrap();
        // Should use all defaults
        assert!(config.source_dir.is_none());
        assert!(config.target_dir.is_none());
        assert!(config.dest_template.is_none());
        assert!(!config.camera_dirs.is_empty()); // default cameras
        assert!(config.get_dest_dir("Canon EOS R6").is_some());
    }

    #[test]
    fn test_parse_only_dirs_section() {
        let toml = r#"
[dirs]
source = "/media/test"
target = "/home/test/Pictures"
template = "{camera}/{year}"
"#;
        let config = parse_config_str(toml).unwrap();
        assert_eq!(config.source_dir, Some(PathBuf::from("/media/test")));
        assert_eq!(
            config.target_dir,
            Some(PathBuf::from("/home/test/Pictures"))
        );
        assert_eq!(config.dest_template, Some("{camera}/{year}".to_string()));
        // Should still have default cameras
        assert!(config.get_dest_dir("Canon EOS R6").is_some());
    }

    #[test]
    fn test_parse_only_cameras_section() {
        let toml = r#"
[cameras]
"Sony A7" = "Sony-images"
"#;
        let config = parse_config_str(toml).unwrap();
        // Should use default dirs (None)
        assert!(config.source_dir.is_none());
        assert!(config.target_dir.is_none());
        // Should use custom cameras, NOT defaults
        assert_eq!(config.get_dest_dir("Sony A7 III"), Some("Sony-images"));
        assert!(config.get_dest_dir("Canon EOS R6").is_none()); // no default cameras
    }

    #[test]
    fn test_parse_both_sections() {
        let toml = r#"
[dirs]
source = "/mnt/sdcard"

[cameras]
"Nikon Z6" = "Nikon-images"
"Nikon Z8" = "Nikon-Z8-images"
"#;
        let config = parse_config_str(toml).unwrap();
        assert_eq!(config.source_dir, Some(PathBuf::from("/mnt/sdcard")));
        assert!(config.target_dir.is_none()); // not specified
        assert_eq!(config.get_dest_dir("Nikon Z6 II"), Some("Nikon-images"));
        assert_eq!(config.get_dest_dir("Nikon Z8"), Some("Nikon-Z8-images"));
    }

    #[test]
    fn test_parse_partial_dirs_section() {
        let toml = r#"
[dirs]
source = "/media/card"
# target not specified
"#;
        let config = parse_config_str(toml).unwrap();
        assert_eq!(config.source_dir, Some(PathBuf::from("/media/card")));
        assert!(config.target_dir.is_none());
    }

    #[test]
    fn test_parse_sections_any_order() {
        // cameras before dirs - should work the same
        let toml = r#"
[cameras]
"Fuji X-T5" = "Fuji-images"

[dirs]
target = "/archive/photos"
"#;
        let config = parse_config_str(toml).unwrap();
        assert!(config.source_dir.is_none());
        assert_eq!(config.target_dir, Some(PathBuf::from("/archive/photos")));
        assert_eq!(config.get_dest_dir("Fuji X-T5"), Some("Fuji-images"));
    }

    #[test]
    fn test_parse_cameras_sorted_by_length() {
        let toml = r#"
[cameras]
"R6" = "short-match"
"EOS R6" = "long-match"
"#;
        let config = parse_config_str(toml).unwrap();
        // Longer match should win
        assert_eq!(config.get_dest_dir("Canon EOS R6"), Some("long-match"));
    }
}
