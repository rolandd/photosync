# Photosync

A personal Rust utility for syncing photos from camera memory cards to
organized directories based on EXIF metadata.

> **Note:** This is a personal project tailored for my specific
> photography workflow. It has defaults for my cameras (Canon EOS R6,
> Canon 6D, GoPro Hero8/Hero13) and file organization style but is
> configurable for other setups.

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

## Installation

Requires Rust 2024 edition.

```bash
cargo build --release
```

The binary will be at `target/release/photosync`.

## Usage

```bash
# Run with defaults (scans /media/$USER, copies to ~/Pictures)
photosync

# Dry run to see what would be copied
photosync -n

# Override source and target directories
photosync --source /path/to/card --target /path/to/photos

# Disable TUI for piping/scripting
photosync --no-tui
```

### Command-Line Options

| Option | Description |
|--------|-------------|
| `-n`, `--dry-run` | Print what would be done without copying |
| `--no-tui` | Disable interactive UI, use plain text output |
| `--source <PATH>` | Override source directory |
| `--target <PATH>` | Override target directory |

## Configuration

Create `photosync.toml` in the current directory or
`~/.config/photosync/photosync.toml`:

```toml
# Optional: Override default paths
[dirs]
source = "/media/roland"
target = "/home/roland/Pictures"

# Map camera model substrings to destination folders
[cameras]
"EOS R6" = "CanonR6-images"
"6D" = "Canon6D-images"
"Hero13" = "GoPro-Hero13"
"HERO13" = "GoPro-Hero13"
```

Camera matching uses longest-match-first semantics, so more specific
patterns take precedence.

## Dependencies

- [nom-exif](https://github.com/nickel-org/nom-exif) - EXIF parsing
- [clap](https://github.com/clap-rs/clap) - Command-line argument parsing
- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI
- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam) - Concurrent pipeline channels
- [walkdir](https://github.com/BurntSushi/walkdir) - Recursive directory traversal
- [chrono](https://github.com/chronotope/chrono) - Date/time handling

## License

MIT License - Copyright 2026 Roland Dreier <roland@kernel.org>
