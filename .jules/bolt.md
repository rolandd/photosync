## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-19 - [File System Traversal Optimization]
**Learning:** File system traversal in `WalkDir` can be a significant bottleneck if `to_string_lossy()` is called on every entry inside the `filter_entry` hot loop. It introduces unnecessary memory allocation and UTF-8 validation overhead for each file name.
**Action:** Pre-allocate `OsString` vectors outside the loop and compare directly against raw `OsStr` references using `e.file_name()` for much faster filtering.
