// SPDX-License-Identifier: MIT
// Copyright 2026 Roland Dreier <roland@kernel.org>

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

    /// Create a starter configuration file
    #[arg(long)]
    pub init: bool,
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

/// Expands supported variables in a path string.
///
/// Supported variables (both `$VAR` and `${VAR}` syntax):
/// - `$HOME` / `${HOME}` - User's home directory from environment
/// - `$XDG_PICTURES_DIR` / `${XDG_PICTURES_DIR}` - User's pictures directory (via dirs crate)
///
/// Unknown variables are left unexpanded.
fn expand_path(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();

    // If no $ in path, return as-is (common case optimization)
    if !s.contains('$') {
        return path;
    }

    let mut result = s.into_owned();

    // Expand $HOME / ${HOME} using dirs crate for cross-platform support
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        result = result.replace("${HOME}", &home_str);
        result = result.replace("$HOME", &home_str);
    }

    // Expand $XDG_PICTURES_DIR / ${XDG_PICTURES_DIR} using dirs crate
    if let Some(pictures) = dirs::picture_dir() {
        let pictures_str = pictures.to_string_lossy();
        result = result.replace("${XDG_PICTURES_DIR}", &pictures_str);
        result = result.replace("$XDG_PICTURES_DIR", &pictures_str);
    }

    PathBuf::from(result)
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

    // Convert HashMap to sorted Vec (empty if not specified)
    let mut camera_dirs = Vec::new();
    if !raw.cameras.is_empty() {
        for (model, dest_dir) in raw.cameras {
            is_safe_path(&dest_dir).with_context(|| {
                format!(
                    "Invalid camera directory mapping for '{}': '{}'",
                    model, dest_dir
                )
            })?;
            camera_dirs.push((model, dest_dir));
        }
    }
    sort_camera_dirs(&mut camera_dirs);

    // Validate template if present
    let dest_template = match raw.dirs.template {
        Some(t) => Some(validate_template(t)?),
        None => None,
    };

    Ok(Config {
        camera_dirs,
        source_dir: raw.dirs.source.map(expand_path),
        target_dir: raw.dirs.target.map(expand_path),
        dest_template,
    })
}

/// Helper to validate if a directory path is safe.
/// Returns Result::Ok if safe, Result::Err if unsafe.
fn is_safe_path(path_str: &str) -> Result<()> {
    if path_str.contains('\\') {
        anyhow::bail!("Backslashes are not allowed. Please use forward slashes '/' for portability.");
    }

    let path = Path::new(path_str);
    if path.is_absolute() {
        anyhow::bail!("Path must be relative.");
    }
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        anyhow::bail!("Path cannot contain '..' (parent directory traversal).");
    }
    Ok(())
}

/// Validates a destination template string.
/// Returns `Ok(template)` if valid, `Err` if invalid.
pub fn validate_template(template: String) -> Result<String> {
    if template.contains('\\') {
        anyhow::bail!(
            "Template '{}' contains backslashes. Please use forward slashes '/' for portability.",
            template
        );
    }

    // Check for path traversal attempts
    if template.split('/').any(|c| c == "..") {
        anyhow::bail!(
            "Template '{}' contains unsafe '..' path traversal.",
            template
        );
    }
    Ok(template)
}

/// Returns the path to the user's config file.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("photosync/photosync.toml"))
}

/// Generate a starter configuration file template.
pub fn generate_config_template() -> String {
    r#"# Photosync Configuration
# Docs: https://github.com/rolandd/photosync#configuration

[dirs]
# source = "/media/your_username"   # Where to scan for photos
# target = "$XDG_PICTURES_DIR"       # Where to copy photos
# template = "{camera}/{year}/{month}/{day}"  # Directory structure
#
# Supported variables (use $VAR or ${VAR} syntax):
#   $HOME             - Your home directory
#   $XDG_PICTURES_DIR - Your Pictures directory (e.g., ~/Pictures)

[cameras]
# Map camera model substrings to folder names.
# The tool matches the longest substring first.
# Examples:
# "EOS R6" = "Canon-R6"
# "Sony A7" = "Sony-A7"
# "iPhone" = "iPhone"
# "Pixel" = "Pixel"
"#
    .to_string()
}

impl Config {
    /// Creates an empty Config for testing.
    #[cfg(test)]
    fn with_defaults() -> Self {
        Self {
            camera_dirs: Vec::new(),
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
    fn test_get_dest_dir_matching() {
        // Create config with custom camera mappings
        let mut config = Config::default();
        config.camera_dirs = vec![
            ("EOS R6".to_string(), "CanonR6-images".to_string()),
            ("6D".to_string(), "Canon6D-images".to_string()),
        ];

        assert_eq!(config.get_dest_dir("Canon EOS R6"), Some("CanonR6-images"));
        assert_eq!(config.get_dest_dir("Canon EOS 6D"), Some("Canon6D-images"));
        assert_eq!(config.get_dest_dir("Sony A7 III"), None);
    }

    #[test]
    fn test_get_dest_dir_empty_config() {
        let config = Config::with_defaults();
        // Empty config should have no camera mappings
        assert!(config.camera_dirs.is_empty());
        assert_eq!(config.get_dest_dir("Any Camera"), None);
    }

    // TOML parsing tests

    #[test]
    fn test_parse_empty_config() {
        let config = parse_config_str("").unwrap();
        // Empty config should have no defaults
        assert!(config.source_dir.is_none());
        assert!(config.target_dir.is_none());
        assert!(config.dest_template.is_none());
        assert!(config.camera_dirs.is_empty()); // no default cameras
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
        // No default cameras, should be empty
        assert!(config.camera_dirs.is_empty());
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

    #[test]
    fn test_parse_config_rejects_unsafe_paths() {
        // Now returns Error instead of silently skipping
        let toml = r#"
[cameras]
"Unsafe1" = "/abs/path"
"#;
        assert!(parse_config_str(toml).is_err());
    }

    #[test]
    fn test_parse_config_rejects_backslashes() {
        let toml = r#"
[cameras]
"Windows" = "folder\\subfolder"
"#;
        let err = parse_config_str(toml).unwrap_err();
        // Check the debug output or iterate sources to find the specific error
        assert!(format!("{:?}", err).contains("Backslashes are not allowed"));
    }

    #[test]
    fn test_is_safe_path() {
        assert!(is_safe_path("simple").is_ok());
        assert!(is_safe_path("nested/dir").is_ok());

        // Unsafe paths
        assert!(is_safe_path("/absolute").is_err());
        assert!(is_safe_path("../parent").is_err());
        assert!(is_safe_path("nested/../parent").is_err());

        // Backslashes
        assert!(is_safe_path(r"nested\dir").is_err());
    }

    #[test]
    fn test_validate_template_valid() {
        assert!(validate_template("{camera}/{year}".to_string()).is_ok());
        assert!(validate_template("{year}/{month}/{day}".to_string()).is_ok());
    }

    #[test]
    fn test_validate_template_unsafe_traversal() {
        // Templates with .. should be rejected
        assert!(validate_template("../{camera}".to_string()).is_err());
        assert!(validate_template("{camera}/../escape".to_string()).is_err());
    }

    #[test]
    fn test_validate_template_backslashes() {
        assert!(validate_template(r"{camera}\{year}".to_string()).is_err());
    }

    #[test]
    fn test_parse_config_rejects_unsafe_template() {
        let toml = r#"
[dirs]
template = "../escape/{camera}"
"#;
        assert!(parse_config_str(toml).is_err());
    }

    // Path expansion tests

    #[test]
    fn test_expand_path_no_variables() {
        let path = PathBuf::from("/some/path/without/variables");
        assert_eq!(expand_path(path.clone()), path);
    }

    #[test]
    fn test_expand_path_home() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            expand_path(PathBuf::from("$HOME/Pictures")),
            home.join("Pictures")
        );
        assert_eq!(
            expand_path(PathBuf::from("${HOME}/Photos")),
            home.join("Photos")
        );
    }

    #[test]
    fn test_expand_path_xdg_pictures_dir() {
        if let Some(pictures) = dirs::picture_dir() {
            assert_eq!(
                expand_path(PathBuf::from("$XDG_PICTURES_DIR")),
                pictures.clone()
            );
            assert_eq!(
                expand_path(PathBuf::from("${XDG_PICTURES_DIR}/archive")),
                pictures.join("archive")
            );
        }
    }

    #[test]
    fn test_expand_path_combined() {
        let home = dirs::home_dir().unwrap();
        let home_str = home.to_string_lossy();
        // Path with HOME followed by literal text
        assert_eq!(
            expand_path(PathBuf::from("${HOME}backup")),
            PathBuf::from(format!("{}backup", home_str))
        );
    }

    #[test]
    fn test_parse_config_expands_dirs() {
        let home = dirs::home_dir().unwrap();
        let toml = r#"
[dirs]
source = "$HOME/media"
target = "${HOME}/Pictures"
"#;
        let config = parse_config_str(toml).unwrap();
        assert_eq!(config.source_dir, Some(home.join("media")));
        assert_eq!(config.target_dir, Some(home.join("Pictures")));
    }
}
