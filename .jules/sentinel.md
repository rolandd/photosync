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

## 2024-02-13 - Terminal Injection via System Error Wrappers
**Vulnerability:** The application was vulnerable to terminal injection attacks when displaying system errors (like `io::Error`). The vulnerability occurred because `report_error` only sanitized standard file strings and failed to sanitize error strings. If a user provided a malicious file path containing ANSI escape sequences, operations on that file could fail, and the system error message would include the raw, unsanitized malicious filename (e.g., `Failed to open source: No such file or directory (os error 2) "malicious\x1b[2Jfile"`).
**Learning:** Terminal injection can happen indirectly. Even if direct outputs (like filenames in UI elements) are sanitized, standard library errors and custom error strings that incorporate those filenames must also be sanitized before reaching the terminal. The assumption that system-generated strings are safe is flawed if they wrap unsanitized user inputs.
**Prevention:** Always apply the UI display sanitization function (e.g., `paths::sanitize_str`) to any text string that will be printed to the terminal, including error messages, logging outputs, and status updates, especially when dealing with file paths.
