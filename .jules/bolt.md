## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-19 - [Directory Traversal Overhead]
**Learning:** Calling `to_string_lossy()` on `OsStr` within a hot directory traversal loop (like `walkdir`'s `filter_entry`) introduces significant overhead due to repeated UTF-8 validation and potential string allocation.
**Action:** Always pre-allocate comparison arrays as `OsString` (or `&OsStr`) outside the loop and perform direct `OsStr` comparisons against `DirEntry::file_name()` instead of casting every entry to a `String` or `Cow<str>`.
