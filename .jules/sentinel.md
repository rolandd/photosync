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

## 2026-02-04 - Partial File Artifacts on Copy Failure
**Vulnerability:** When using `io::copy` to a `create_new(true)` destination, a failure mid-stream left a partially written file. This file was not cleaned up, causing subsequent runs to see it as "existing" and skip it, potentially leading to data loss if the source is deleted.
**Learning:** `io::copy` is not transactional. If it fails, the destination file remains in its current state.
**Prevention:** Catch errors from `io::copy` and explicitly delete the destination file. On Windows, ensure the file handle is dropped before deletion.
