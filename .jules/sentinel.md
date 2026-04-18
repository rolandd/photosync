## 2026-02-04 - TOCTOU Vulnerability in fs::copy
**Vulnerability:** A Time-of-Check Time-of-Use (TOCTOU) race condition was identified in the file copying logic. The application checked if a destination file existed before copying, but `fs::copy` was not atomic with this check. An attacker could create a symlink at the destination between the check and the copy, causing the application to overwrite an arbitrary file (the symlink target) with the user's permissions.
**Learning:** `fs::copy` in Rust (and many other languages) implicitly follows symlinks at the destination if the file exists. Using `dest_path.exists()` is insufficient to prevent overwriting.
**Prevention:** Use `fs::OpenOptions` with `.create_new(true)` to atomically create and open the destination file. If it exists (or is a symlink), the operation fails, preventing the race. Use `io::copy` to transfer data to the newly created file handle.

## 2026-02-04 - Unsafe Permission Propagation in File Copy
**Vulnerability:** The `atomic_copy` function blindly copied all permission bits from the source file to the destination using `set_permissions`. This allowed files from FAT filesystems (which often appear as world-executable) to be copied as executable files to the user's library.
**Learning:** "Best effort" metadata copying must be sanitized. Copying permission bits without filtering can introduce security risks (e.g., executable images).
**Prevention:** When copying files, either rely on the default umask (don't copy permissions) or explicitly mask out dangerous bits (like `0o111`) if preserving other attributes (like read-only status) is required.

## 2026-02-04 - Terminal Injection via Filenames
**Vulnerability:** Filenames and EXIF data containing ANSI escape codes or control characters were displayed raw in the TUI and text logs, allowing potential terminal manipulation.
**Learning:** `Path::display()` does not sanitize control characters. It only escapes invalid UTF-8 (using replacement characters), but valid UTF-8 control codes are passed through.
**Prevention:** Always sanitize user-controlled strings (filenames, metadata) before displaying them in a terminal. Use a helper function to replace control characters with a safe placeholder (like `?`).

## 2026-03-07 - Incomplete Cleanup on Failed Copy (CWE-459)
**Vulnerability:** A failed `io::copy` inside `atomic_copy_file` left partially written files at the destination. Because the pipeline handles `io::ErrorKind::AlreadyExists` by treating it as a duplicate, a failed copy attempt would permanently prevent the photo from being synced on subsequent runs (creating a persistent DoS/data loss condition).
**Learning:** System APIs like `io::copy` do not guarantee state rollback on failure. When implementing atomic file operations with `create_new(true)`, the application is responsible for cleaning up artifacts if the operation aborts mid-stream.
**Prevention:** Always wrap `io::copy` in a `match` block. On `Err`, explicitly `drop()` the destination file handle (crucial for Windows where open files are locked) and remove the incomplete file using `fs::remove_file()`.

## 2024-04-18 - Prevent DoS from blocking I/O on special files
**Vulnerability:** A Time-of-Check Time-of-Use (TOCTOU) / Blocking I/O vulnerability existed where `File::open` was called on unsanitized file paths before `fs::metadata` checked if the file was a regular file (e.g., in `file_handler`, `file_processor`, and `FileComparator::compare_file`). If the file was a FIFO or device, `File::open` would block indefinitely.
**Learning:** Checking `is_file()` *after* `File::open()` is too late, as opening special files like FIFOs can block the calling thread or cause DoS.
**Prevention:** Always use `fs::metadata` or `fs::symlink_metadata` to verify that a file path points to a regular file *before* attempting to open it, especially when iterating over files from a user-controlled or external source.
