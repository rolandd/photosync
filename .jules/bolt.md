## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.
## 2026-05-18 - [File Traversal Bottleneck]
**Learning:** Calling `to_string_lossy()` on `OsStr` for every file and directory entry during `WalkDir` traversal introduces significant allocation and UTF-8 validation overhead, creating a bottleneck.
**Action:** Pre-allocate target strings as `OsString` vectors outside the traversal loop and perform direct `OsStr` comparisons inside `filter_entry` (e.g., `e.file_name() == os_str`) to eliminate this overhead.
