//! Pipeline orchestration for file processing.
//!
//! This module manages the multi-threaded pipeline:
//! 1. **Walker**: Discovers files in source directories
//! 2. **Processor**: Extracts EXIF metadata
//! 3. **Handler**: Copies files to destination with deduplication

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use chrono::NaiveDate;
use nom_exif::{ExifIter, ExifTag, MediaParser, MediaSource};
use walkdir::WalkDir;

use crate::config::Config;
use crate::paths::{self, DestPath, SourcePath};
use crate::progress::ProgressMsg;

/// Channel buffer size for pipeline stages.
const CHANNEL_BUFFER_SIZE: usize = 1024;

/// Represents a file with complete EXIF metadata.
#[derive(Debug)]
pub struct FileInfo {
    pub path: SourcePath,
    pub model: String,
    pub date: NaiveDate,
}

/// Spawns the processing pipeline threads.
///
/// This function sets up the channels and spawns the worker threads:
/// 1. Walker: Scans the source directory.
/// 2. Processor: Extracts EXIF data.
/// 3. Handler: Copies files to the destination.
/// 4. Monitor: Waits for all workers to finish and sends the Done signal.
///
/// Returns the `JoinHandle` of the monitor thread.
pub fn spawn_pipeline(
    source_dir: PathBuf,
    target_dir: PathBuf,
    config: Config,
    dry_run: bool,
    progress_tx: SyncSender<ProgressMsg>,
) -> thread::JoinHandle<()> {
    // Create pipeline channels
    let (walker_tx, processor_rx) = mpsc::sync_channel::<PathBuf>(CHANNEL_BUFFER_SIZE);
    let (processor_tx, handler_rx) = mpsc::sync_channel::<FileInfo>(CHANNEL_BUFFER_SIZE);

    // Clone senders for each worker
    let walker_progress = progress_tx.clone();
    let processor_progress = progress_tx.clone();
    let handler_progress = progress_tx.clone();

    // Spawn the producer thread (file walker)
    // Extract exclude_dirs to pass to walker
    let exclude_dirs = config.exclude_dirs.clone();
    let walker_handle = thread::spawn(move || {
        file_walker(source_dir, exclude_dirs, walker_tx, walker_progress);
    });

    // Spawn the processor thread (EXIF extraction)
    let processor_handle = thread::spawn(move || {
        file_processor(processor_rx, processor_tx, processor_progress);
    });

    // Spawn the handler thread (final processing)
    // We clone config for the handler since it needs to look up camera dirs
    let handler_config = config.clone();
    let handler_handle = thread::spawn(move || {
        file_handler(
            handler_rx,
            target_dir,
            handler_config,
            dry_run,
            handler_progress,
        );
    });

    // Spawn a thread to send Done after all workers finish
    let done_tx = progress_tx;
    thread::spawn(move || {
        // Collect thread handles and their names for error reporting
        let handles = [
            (walker_handle, "Walker"),
            (processor_handle, "Processor"),
            (handler_handle, "Handler"),
        ];

        for (handle, name) in handles {
            if let Err(e) = handle.join() {
                eprintln!("{name} thread panicked: {e:?}");
            }
        }

        // Send Done (ignoring send errors if receiver is gone)
        let _ = done_tx.send(ProgressMsg::Done);
    })
}

/// Helper to send progress messages, suppressing errors.
fn send_progress(tx: &SyncSender<ProgressMsg>, msg: ProgressMsg) {
    if let Err(_e) = tx.send(msg) {
        #[cfg(debug_assertions)]
        eprintln!("Warning: Failed to send progress message: {:?}", _e);
    }
}

/// Helper to report errors to the UI
fn report_error(
    tx: &SyncSender<ProgressMsg>,
    filename: impl Into<String>,
    error: impl Into<String>,
) {
    send_progress(
        tx,
        ProgressMsg::CopyError {
            filename: filename.into(),
            error: error.into(),
        },
    );
}

/// Chunk size for file comparison (128 KB).
const FILE_COMPARE_CHUNK_SIZE: usize = 128 * 1024;

/// Helper struct to compare files reusing internal buffers.
struct FileComparator {
    buf1: Vec<u8>,
    buf2: Vec<u8>,
}

impl FileComparator {
    fn new() -> Self {
        Self {
            buf1: Vec::new(),
            buf2: Vec::new(),
        }
    }

    /// Compare an open file with another file path for equality.
    ///
    /// **Note:** The caller must ensure that `file1` is at the beginning of the file (position 0).
    fn compare_file(&mut self, file1: &mut File, size1: u64, path2: &Path) -> io::Result<bool> {
        // Fast path: compare sizes first
        let meta2 = fs::metadata(path2)?;
        if size1 != meta2.len() {
            return Ok(false);
        }

        // Ensure buffers are the correct size
        if self.buf1.len() != FILE_COMPARE_CHUNK_SIZE {
            self.buf1.resize(FILE_COMPARE_CHUNK_SIZE, 0);
        }
        if self.buf2.len() != FILE_COMPARE_CHUNK_SIZE {
            self.buf2.resize(FILE_COMPARE_CHUNK_SIZE, 0);
        }

        let mut file2 = File::open(path2)?;

        loop {
            let n1 = Self::read_chunk(file1, &mut self.buf1)?;
            let n2 = Self::read_chunk(&mut file2, &mut self.buf2)?;

            if n1 != n2 || self.buf1[..n1] != self.buf2[..n2] {
                return Ok(false);
            }

            if n1 == 0 {
                return Ok(true);
            }
        }
    }

    /// Helper to read a chunk from a file until buffer is full or EOF.
    fn read_chunk(reader: &mut File, buf: &mut [u8]) -> io::Result<usize> {
        let mut read = 0;
        while read < buf.len() {
            let n = reader.read(&mut buf[read..])?;
            if n == 0 {
                break;
            }
            read += n;
        }
        Ok(read)
    }
}

/// Discovers all files under source directories.
fn file_walker(
    source_dir: PathBuf,
    exclude_dirs: Vec<String>,
    tx: SyncSender<PathBuf>,
    progress_tx: SyncSender<ProgressMsg>,
) {
    send_progress(
        &progress_tx,
        ProgressMsg::ScanningDir(SourcePath::new(source_dir.clone())),
    );

    let walker = WalkDir::new(&source_dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(move |e: &walkdir::DirEntry| {
            let name = e.file_name().to_string_lossy();
            !exclude_dirs.iter().any(|ex| name == *ex)
        });

    for walk_entry in walker {
        match walk_entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    let path = entry.into_path();
                    send_progress(&progress_tx, ProgressMsg::FileFound);
                    if tx.send(path).is_err() {
                        return;
                    }
                }
            }
            Err(e) => {
                let path = e.path().unwrap_or(Path::new("?")).to_path_buf();
                send_progress(
                    &progress_tx,
                    ProgressMsg::ScanError {
                        path: SourcePath::new(path),
                        error: paths::sanitize_str(&e.to_string()),
                    },
                );
            }
        }
    }

    send_progress(&progress_tx, ProgressMsg::ScanComplete);
}

/// Processes file paths received from the channel.
fn file_processor(
    rx: Receiver<PathBuf>,
    tx: SyncSender<FileInfo>,
    progress_tx: SyncSender<ProgressMsg>,
) {
    let mut parser = MediaParser::new();

    for path in rx {
        let Ok(ms) = MediaSource::file_path(&path) else {
            continue;
        };

        if !ms.has_exif() {
            continue;
        }

        let iter: ExifIter = match parser.parse(ms) {
            Ok(iter) => iter,
            Err(_) => continue,
        };

        let mut model = None;
        let mut date = None;

        for mut entry in iter {
            if let Some(tag) = entry.tag() {
                match tag {
                    ExifTag::Model => {
                        model = entry
                            .take_value()
                            .and_then(|v| v.as_str().map(paths::sanitize_str));
                    }
                    ExifTag::DateTimeOriginal => {
                        date = entry
                            .take_value()
                            .and_then(|v| v.as_time_components())
                            .map(|(ndt, _offset)| ndt.date());
                    }
                    _ => {}
                }
            }

            if model.is_some() && date.is_some() {
                break;
            }
        }

        if let (Some(model), Some(date)) = (model, date) {
            send_progress(
                &progress_tx,
                ProgressMsg::ExifExtracted {
                    path: SourcePath::new(path.clone()),
                    model: model.clone(),
                },
            );
            let file_info = FileInfo {
                path: SourcePath::new(path),
                model,
                date,
            };
            if tx.send(file_info).is_err() {
                return;
            }
        }
    }
}

/// Result of computing a destination directory.
#[derive(Debug, PartialEq, Eq)]
enum DestDirResult {
    Ok(PathBuf),
    UnknownCamera,
    TemplateError(String),
}

/// Helper to compute the destination directory for a file.
/// Returns `DestDirResult::UnknownCamera` if the camera model is unknown (when template needs it),
/// `DestDirResult::TemplateError` if the template is invalid, or `DestDirResult::Ok` with the path.
fn compute_dest_dir(
    target_dir: &Path,
    config: &Config,
    model: &str,
    date: NaiveDate,
) -> DestDirResult {
    let template = config
        .dest_template
        .as_deref()
        .unwrap_or("{camera}/{year}/{month}/{day}");

    // Only look up camera if template uses it
    let camera_dir_name = if template.contains("{camera}") {
        match config.get_dest_dir(model) {
            Some(name) => name,
            None => return DestDirResult::UnknownCamera,
        }
    } else {
        "" // Not used
    };

    // Note: camera_dir_name is already validated when loading config (no absolute paths or '..')

    let mut path_str = template.to_string();

    // Replacements
    // Note: We perform simple replacements.
    path_str = path_str.replace("{camera}", camera_dir_name);
    path_str = path_str.replace("{year}", &date.format("%Y").to_string());
    path_str = path_str.replace("{month}", &date.format("%m").to_string());
    path_str = path_str.replace("{day}", &date.format("%d").to_string());

    // Basic validation: Check for remaining curly braces which might indicate broken tags
    if path_str.contains('{') || path_str.contains('}') {
        return DestDirResult::TemplateError(format!(
            "Malformed template or unknown tag in '{template}'. Result: '{path_str}'"
        ));
    }

    // Construct the full path
    // We split by '/' to handle the template structure.
    // Even if the replacement values contain path separators (e.g. camera_dir_name="A/B"),
    // split('/') will break them down correctly for PathBuf::push.
    // We also check for '..' components dynamically.

    let mut dest_path = target_dir.to_path_buf();
    for component in path_str.split('/').filter(|c| !c.is_empty()) {
        if component == ".." || component == "." {
            // ".." is unsafe. "." is redundant but technically safe, but let's avoid it to be clean.
            if component == ".." {
                return DestDirResult::TemplateError(format!(
                    "Template resulted in '..' component, which is unsafe. Path: '{path_str}'"
                ));
            }
            continue;
        }

        // On Windows, checking for drive letters in components
        if component.contains(':') {
            // Rudimentary check for drive letters or absolute paths injected via components
            // (e.g. if replacement was "C:\Bad")
            if Path::new(component).is_absolute() {
                return DestDirResult::TemplateError(format!(
                    "Component '{component}' appears to be absolute. Path: '{path_str}'"
                ));
            }
        }

        dest_path.push(component);
    }

    DestDirResult::Ok(dest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, Write};
    use std::path::Path;

    /// Helper to create a test file with given content.
    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::File::create(&path)
            .unwrap()
            .write_all(content)
            .unwrap();
        path
    }

    #[test]
    fn test_files_are_equal_identical() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "file1.txt", b"Hello, world!");
        let file2 = create_test_file(dir.path(), "file2.txt", b"Hello, world!");

        let mut comparator = FileComparator::new();
        let mut f1 = File::open(&file1).unwrap();
        let len = f1.metadata().unwrap().len();
        assert!(comparator.compare_file(&mut f1, len, &file2).unwrap());
    }

    #[test]
    fn test_files_are_equal_different_size() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "file1.txt", b"short");
        let file2 = create_test_file(dir.path(), "file2.txt", b"much longer content");

        let mut comparator = FileComparator::new();
        let mut f1 = File::open(&file1).unwrap();
        let len = f1.metadata().unwrap().len();
        assert!(!comparator.compare_file(&mut f1, len, &file2).unwrap());
    }

    #[test]
    fn test_files_are_equal_same_size_different_content() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "file1.txt", b"AAAA");
        let file2 = create_test_file(dir.path(), "file2.txt", b"BBBB");

        let mut comparator = FileComparator::new();
        let mut f1 = File::open(&file1).unwrap();
        let len = f1.metadata().unwrap().len();
        assert!(!comparator.compare_file(&mut f1, len, &file2).unwrap());
    }

    #[test]
    fn test_files_are_equal_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = create_test_file(dir.path(), "exists.txt", b"");
        let file2 = dir.path().join("missing.txt");

        let mut comparator = FileComparator::new();
        let mut f1 = File::open(&file1).unwrap();
        let len = f1.metadata().unwrap().len();
        assert!(comparator.compare_file(&mut f1, len, &file2).is_err());
    }

    #[test]
    fn test_compute_dest_dir_default() {
        // config needs to be created manually since it's in another module
        let mut config = Config::default();
        config.camera_dirs = vec![("Model".to_string(), "CameraDir".to_string())];
        // default template is implied when None

        let target = PathBuf::from("/target");
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();

        let DestDirResult::Ok(dest) = compute_dest_dir(&target, &config, "Model", date) else {
            panic!("Expected DestDirResult::Ok");
        };
        // Use chained joins for cross-platform compatibility
        assert_eq!(
            dest,
            target.join("CameraDir").join("2023").join("10").join("25")
        );
    }

    #[test]
    fn test_compute_dest_dir_custom_template() {
        let mut config = Config::default();
        config.camera_dirs = vec![("Model".to_string(), "CameraDir".to_string())];
        config.dest_template = Some("{year}-{month}/{camera}".to_string());

        let target = PathBuf::from("/target");
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();

        let DestDirResult::Ok(dest) = compute_dest_dir(&target, &config, "Model", date) else {
            panic!("Expected DestDirResult::Ok");
        };
        // Use chained joins for cross-platform compatibility
        assert_eq!(dest, target.join("2023-10").join("CameraDir"));
    }

    #[test]
    fn test_compute_dest_dir_unknown_camera() {
        let config = Config::default(); // empty camera_dirs
        let target = PathBuf::from("/target");
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();

        assert!(matches!(
            compute_dest_dir(&target, &config, "Unknown", date),
            DestDirResult::UnknownCamera
        ));
    }

    #[test]
    fn test_compute_dest_dir_malformed_template() {
        let mut config = Config::default();
        config.camera_dirs = vec![("Model".to_string(), "CameraDir".to_string())];
        config.dest_template = Some("{camera}/{year}/{unknown_tag}".to_string());

        let target = PathBuf::from("/target");
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();

        // Should return TemplateError because of unknown tag left in string
        assert!(matches!(
            compute_dest_dir(&target, &config, "Model", date),
            DestDirResult::TemplateError(_)
        ));
    }

    #[test]
    fn test_compute_dest_dir_unsafe_path() {
        let mut config = Config::default();
        config.camera_dirs = vec![("Model".to_string(), "CameraDir".to_string())];
        config.dest_template = Some("../{camera}".to_string());

        let target = PathBuf::from("/target");
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();

        // Should return TemplateError because of ".."
        assert!(matches!(
            compute_dest_dir(&target, &config, "Model", date),
            DestDirResult::TemplateError(_)
        ));
    }

    #[test]
    fn test_compute_dest_dir_no_camera_in_template() {
        // Template without {camera} should work even with no camera mappings
        let mut config = Config::default();
        config.dest_template = Some("{year}/{month}/{day}".to_string());
        // No camera_dirs configured

        let target = PathBuf::from("/target");
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();

        let DestDirResult::Ok(dest) = compute_dest_dir(&target, &config, "Any Camera", date) else {
            panic!("Expected DestDirResult::Ok");
        };
        assert_eq!(dest, target.join("2023").join("10").join("25"));
    }

    #[test]
    fn test_files_are_equal_large_file() {
        let dir = tempfile::tempdir().unwrap();

        // Create files larger than FILE_COMPARE_CHUNK_SIZE (128 KB)
        let large_content: Vec<u8> = (0..200_000).map(|i| (i % 256) as u8).collect();
        let file1 = create_test_file(dir.path(), "large1.bin", &large_content);
        let file2 = create_test_file(dir.path(), "large2.bin", &large_content);

        let mut comparator = FileComparator::new();
        let mut f1 = File::open(&file1).unwrap();
        let len = f1.metadata().unwrap().len();
        assert!(comparator.compare_file(&mut f1, len, &file2).unwrap());

        // Create a file that differs only in the second chunk
        let mut different_content = large_content.clone();
        different_content[150_000] = 0xFF; // Modify byte in second chunk
        let file3 = create_test_file(dir.path(), "large3.bin", &different_content);

        // Reset f1 for next comparison or open again? compare_file rewinds it.
        f1.seek(std::io::SeekFrom::Start(0)).unwrap();
        assert!(!comparator.compare_file(&mut f1, len, &file3).unwrap());
    }

    #[test]
    fn test_compute_dest_dir_nested_camera_path() {
        let mut config = Config::default();
        // Camera dir with nested path
        config.camera_dirs = vec![("Model".to_string(), "Canon/EOS/R6".to_string())];

        let target = PathBuf::from("/target");
        let date = NaiveDate::from_ymd_opt(2023, 10, 25).unwrap();

        let DestDirResult::Ok(dest) = compute_dest_dir(&target, &config, "Model", date) else {
            panic!("Expected DestDirResult::Ok");
        };
        // Should properly join nested path components
        assert_eq!(
            dest,
            target
                .join("Canon")
                .join("EOS")
                .join("R6")
                .join("2023")
                .join("10")
                .join("25")
        );
    }

    #[test]
    fn test_file_walker_exclude() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();

        // Create "keep" and "ignore" subdirectories
        let keep_dir = src_dir.join("keep");
        let ignore_dir = src_dir.join("ignore");
        std::fs::create_dir(&keep_dir).unwrap();
        std::fs::create_dir(&ignore_dir).unwrap();

        // Create files
        create_test_file(&keep_dir, "good.jpg", b"");
        create_test_file(&ignore_dir, "bad.jpg", b"");

        let (tx, rx) = std::sync::mpsc::sync_channel(10);
        let (progress_tx, _) = std::sync::mpsc::sync_channel(10);

        // Run walker with "ignore" in exclude list
        file_walker(src_dir, vec!["ignore".to_string()], tx, progress_tx);

        // Collect results
        let mut paths = Vec::new();
        while let Ok(path) = rx.try_recv() {
            paths.push(path);
        }

        // Verify
        assert_eq!(paths.len(), 1);
        assert!(paths[0].to_string_lossy().contains("good.jpg"));
        assert!(!paths[0].to_string_lossy().contains("ignore"));
    }

    #[test]
    #[cfg(unix)]
    fn test_atomic_copy_uses_default_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("source.bin");
        let dest = dir.path().join("dest.bin");

        // Create executable source file
        {
            let mut file = File::create(&src).unwrap();
            file.write_all(b"data").unwrap();
            let mut perms = file.metadata().unwrap().permissions();
            perms.set_mode(0o777); // rwxrwxrwx
            file.set_permissions(perms).unwrap();
        }

        // Run atomic_copy_file
        let mut file = File::open(&src).unwrap();
        let meta = file.metadata().unwrap();
        atomic_copy_file(&mut file, &dest, &meta).unwrap();

        // Check destination permissions
        let dest_perms = fs::metadata(&dest).unwrap().permissions();
        let mode = dest_perms.mode() & 0o777;

        // Verify that we didn't copy 777 permissions
        assert_ne!(mode, 0o777);
        // Verify no execute bits (assuming standard umask isn't 000)
        assert_eq!(mode & 0o111, 0, "Destination file should not be executable");
        // Verify standard read/write permissions for owner (rw-------)
        assert_eq!(
            mode & 0o600,
            0o600,
            "Owner should have read/write permissions"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_atomic_copy_rejects_fifo() {
        use std::process::Command;
        use std::thread;

        let dir = tempfile::tempdir().unwrap();
        let fifo_path = dir.path().join("test_fifo");
        let dest_path = dir.path().join("fifo_copy.bin");

        // Create FIFO using mkfifo command
        let status = Command::new("mkfifo")
            .arg(&fifo_path)
            .status()
            .expect("failed to execute mkfifo");
        assert!(status.success(), "mkfifo failed");

        // Spawn a thread to open the FIFO for writing.
        // Opening a FIFO for reading blocks until a writer opens it.
        // Opening for writing blocks until a reader opens it.
        let fifo_clone = fifo_path.clone();
        let handle = thread::spawn(move || {
            // Open for writing to unblock the reader in main thread.
            // Using OpenOptions to explicitly open for writing.
            if let Ok(mut file) = fs::OpenOptions::new().write(true).open(fifo_clone) {
                let _ = file.write_all(b"data");
            }
        });

        // Attempt to copy from the FIFO.
        // open() will succeed once the writer thread opens it.
        let mut reader = File::open(&fifo_path).expect("Failed to open FIFO");

        // This should fail because it's not a regular file.
        let meta = reader.metadata().unwrap();
        let result = atomic_copy_file(&mut reader, &dest_path, &meta);

        // Ensure writer thread finishes
        let _ = handle.join();

        assert!(result.is_err(), "atomic_copy_file should fail for FIFO");
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), "Source is not a regular file");
    }
}

/// Copies a file to a destination, failing if the destination already exists.
/// This prevents TOCTOU races where a symlink is created at the destination
/// between the existence check and the copy.
///
/// **Note:** The caller must ensure that `reader` is at the beginning of the file (position 0)
/// and that `meta` corresponds to the `reader` file handle.
fn atomic_copy_file(reader: &mut File, dest: &Path, meta: &std::fs::Metadata) -> io::Result<u64> {
    // Security check: ensure we are reading from a regular file, not a device/pipe/socket.
    // This mitigates DoS risks (reading infinite streams like /dev/zero) and blocking on pipes.
    // Uses fstat (cheap).
    if !meta.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source is not a regular file",
        ));
    }

    // Create a temporary file path in the same directory
    let parent = dest
        .parent()
        .ok_or_else(|| io::Error::other("Invalid destination path"))?;
    let file_name = dest
        .file_name()
        .ok_or_else(|| io::Error::other("Invalid destination filename"))?;

    // Use a randomized temp file name to avoid collisions and symlink attacks.
    // We retry a few times in case of collision.
    let pid = std::process::id();
    for i in 0..3 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let temp_name = format!(
            ".{}.{}.{}-{}.part",
            file_name.to_string_lossy(),
            pid,
            nanos,
            i
        );
        let temp_path = parent.join(temp_name);

        // Try to create the temp file securely (create_new=true prevents following symlinks)
        let writer_result = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path);

        match writer_result {
            Ok(mut writer) => {
                // Copy to temp file
                let len = match io::copy(reader, &mut writer) {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = fs::remove_file(&temp_path);
                        return Err(e);
                    }
                };

                // Note: We do NOT copy permissions from the source file.
                // For photos/archives, it's safer to rely on the user's umask and default file creation
                // permissions (typically 0644 or 0600) rather than trusting metadata from the source
                // filesystem (e.g. FAT/exFAT often reports 0777/0755).

                // Try atomic link
                if let Err(link_err) = fs::hard_link(&temp_path, dest) {
                    // Clean up temp file
                    let _ = fs::remove_file(&temp_path);

                    // Check if failure was due to destination existing
                    if link_err.kind() == io::ErrorKind::AlreadyExists || dest.exists() {
                        return Err(io::Error::new(
                            io::ErrorKind::AlreadyExists,
                            "Destination already exists",
                        ));
                    }

                    // Fallback: Hard links might not be supported (e.g. FAT32).
                    // We must rewind the reader and try direct copy.
                    // This sacrifices atomicity but preserves safety (no overwrite).
                    io::Seek::seek(reader, io::SeekFrom::Start(0))?;

                    let mut direct_writer = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(dest)?;
                    return io::copy(reader, &mut direct_writer);
                }

                // Success: remove temp file (it was hard linked)
                let _ = fs::remove_file(&temp_path);
                return Ok(len);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // Collision, retry loop
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Failed to create unique temporary file",
    ))
}

/// Final handler for files with complete metadata.
/// Copies files to `target_dir/<camera_dir>/YYYY/MM/DD/`
fn file_handler(
    rx: Receiver<FileInfo>,
    target_dir: PathBuf,
    config: Config,
    dry_run: bool,
    progress_tx: SyncSender<ProgressMsg>,
) {
    let mut comparator = FileComparator::new();
    let mut last_dest_dir: Option<PathBuf> = None;

    for info in rx {
        let dest_dir = match compute_dest_dir(&target_dir, &config, &info.model, info.date) {
            DestDirResult::Ok(path) => path,
            DestDirResult::UnknownCamera => {
                send_progress(
                    &progress_tx,
                    ProgressMsg::UnknownCamera {
                        model: info.model.clone(),
                    },
                );
                continue;
            }
            DestDirResult::TemplateError(msg) => {
                report_error(&progress_tx, info.path.to_string(), msg);
                continue;
            }
        };

        let Some(filename) = info.path.file_name() else {
            report_error(&progress_tx, info.path.to_string(), "No filename in path");
            continue;
        };
        // Security: Use sanitize_filename to prevent invalid characters and Windows issues
        let filename_str = paths::sanitize_filename(&filename.to_string_lossy());
        // Use sanitized filename for destination to prevent creating files with control characters
        let dest_path = dest_dir.join(&filename_str);

        // Open source file early to reuse handle and avoid double open/stat
        let mut src_file = match File::open(&info.path) {
            Ok(f) => f,
            Err(e) => {
                report_error(
                    &progress_tx,
                    info.path.to_string(),
                    format!("Failed to open source: {e}"),
                );
                continue;
            }
        };

        // Use fstat (cheap) to get size and check if regular file
        let src_meta = match src_file.metadata() {
            Ok(m) => m,
            Err(e) => {
                report_error(
                    &progress_tx,
                    info.path.to_string(),
                    format!("Failed to stat source: {e}"),
                );
                continue;
            }
        };

        if !src_meta.is_file() {
            // Already filtered by walker, but double check for safety
            report_error(
                &progress_tx,
                info.path.to_string(),
                "Source is not a regular file",
            );
            continue;
        }
        let size = src_meta.len();

        if dest_path.exists() {
            // Check if contents are the same reusing src_file
            let is_same = comparator
                .compare_file(&mut src_file, size, &dest_path)
                .unwrap_or_else(|e| {
                    // Log error and assume different (safer)
                    report_error(
                        &progress_tx,
                        filename_str.clone(),
                        format!("Error comparing files: {e}"),
                    );
                    false
                });

            if !is_same {
                // Suspicious: same name but different contents
                send_progress(
                    &progress_tx,
                    ProgressMsg::SuspiciousDuplicate {
                        src: info.path.clone(),
                        dest: DestPath::new(dest_path.clone()),
                    },
                );
            }

            send_progress(
                &progress_tx,
                ProgressMsg::CopySkipped {
                    filename: filename_str,
                },
            );
            continue;
        }

        if dry_run {
            send_progress(
                &progress_tx,
                ProgressMsg::CopyStarted {
                    src: info.path.clone(),
                    dest: DestPath::new(dest_path.clone()),
                    size,
                },
            );
            send_progress(
                &progress_tx,
                ProgressMsg::CopyComplete {
                    filename: filename_str,
                    size,
                    duration: Duration::from_millis(1),
                },
            );
            continue;
        }

        // Create destination directory with caching to avoid redundant calls
        if last_dest_dir.as_ref() != Some(&dest_dir) {
            if let Err(e) = fs::create_dir_all(&dest_dir) {
                report_error(
                    &progress_tx,
                    filename_str,
                    format!("Failed to create directory: {e}"),
                );
                continue;
            }
            last_dest_dir = Some(dest_dir.clone());
        }

        send_progress(
            &progress_tx,
            ProgressMsg::CopyStarted {
                src: info.path.clone(),
                dest: DestPath::new(dest_path.clone()),
                size,
            },
        );

        let start = Instant::now();
        // Use atomic_copy_file with already open file
        if let Err(e) = atomic_copy_file(&mut src_file, &dest_path, &src_meta) {
            report_error(&progress_tx, filename_str, format!("Copy failed: {e}"));
            continue;
        }
        let duration = start.elapsed();

        send_progress(
            &progress_tx,
            ProgressMsg::CopyComplete {
                filename: filename_str,
                size,
                duration,
            },
        );
    }
}
