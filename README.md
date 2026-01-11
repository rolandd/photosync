# Photosync

[![CI](https://github.com/rolandd/photosync/actions/workflows/ci.yml/badge.svg)](https://github.com/rolandd/photosync/actions/workflows/ci.yml)

A Rust utility for syncing photos from camera memory cards to
organized directories based on EXIF metadata.

## What It Does

Photosync scans source directories (typically mounted SD cards at
`/media/<username>`) for photos and videos, extracts EXIF metadata,
and copies files to a date-organized folder structure:

```
~/Pictures/<camera_dir>/YYYY/MM/DD/<original_filename>
```

For example, a photo shot on October 25, 2023 with a Canon EOS R6
would be copied to:
```
~/Pictures/CanonR6-images/2023/10/25/IMG_1234.CR3
```

The directory structure is configurable using a template system.

## Installation

Requires Rust 2024 edition.

```bash
cargo install photosync
# Or build from source:
cargo build --release
```

## Quick Start

```bash
# Create a configuration file
photosync --init

# Edit the config to add your camera mappings
# (location shown by --init output)

# Run a dry run to see what would be copied
photosync -n

# Sync your photos
photosync
```

## Usage

```bash
# Run with defaults (scans /media/$USER, copies to ~/Pictures)
photosync

# Dry run to see what would be copied
photosync -n

# Override source and target directories
photosync --source /path/to/card --target /path/to/photos

# Use a custom directory structure
photosync --template "{year}/{month}/{camera}"

# Disable TUI for piping/scripting
photosync --no-tui
```

### Command-Line Options

| Option | Description |
|--------|-------------|
| `--init` | Create a starter configuration file |
| `-n`, `--dry-run` | Print what would be done without copying |
| `--no-tui` | Disable interactive UI, use plain text output |
| `--source <PATH>` | Override source directory |
| `--target <PATH>` | Override target directory |
| `--template <FMT>`| Override destination directory template |

## Configuration

Create `photosync.toml` in the current directory or
`~/.config/photosync/photosync.toml`:

```toml
# Optional: Override default paths
[dirs]
source = "/media/roland"
target = "/home/roland/Pictures"
template = "{camera}/{year}/{month}/{day}"

# Map camera model substrings to destination folders
[cameras]
"EOS R6" = "CanonR6-images"
"6D" = "Canon6D-images"
"Hero13" = "GoPro-Hero13"
"HERO13" = "GoPro-Hero13"
```

### Template System

The destination directory structure can be customized using the `template` option.
The following tags are available:

- `{camera}`: The matched camera directory name (from config)
- `{year}`: 4-digit year (e.g., "2023")
- `{month}`: 2-digit month (e.g., "10")
- `{day}`: 2-digit day (e.g., "25")

The default template is `{camera}/{year}/{month}/{day}`.

**Note on Safety:**
- Camera directory names defined in the config must be relative paths
  and cannot contain `..` (parent directory traversal).
- The resulting path from the template is also validated to ensure it
  does not attempt to traverse outside the target directory.

Camera matching uses longest-match-first semantics, so more specific
patterns take precedence.

## Dependencies

- [nom-exif](https://github.com/nickel-org/nom-exif) - EXIF parsing
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [walkdir](https://github.com/BurntSushi/walkdir) - Recursive directory traversal
- [chrono](https://github.com/chronotope/chrono) - Date/time handling
- [dirs](https://github.com/dirs-dev/dirs-rs) - Platform-specific standard directories (Pictures, config, etc.)
- [anyhow](https://github.com/dtolnay/anyhow) - Error handling
- [toml](https://github.com/toml-rs/toml) - Configuration file parsing

## License

[MIT License](https://github.com/rolandd/photosync/blob/main/LICENSE) - Copyright 2026 Roland Dreier <roland@kernel.org>
