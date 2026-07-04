## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-18 - [Optimize File Traversal by Avoiding `to_string_lossy()`]
**Learning:** During directory traversal in `file_walker`, using `e.file_name().to_string_lossy()` on every entry incurs overhead due to UTF-8 validation and potential memory allocation. By pre-allocating an `OsString` vector for excluded directories outside the hot loop, we can compare directly against `OsStr` (via `e.file_name()`), which operates ~5x faster.
**Action:** Avoid string conversions inside hot file system traversal loops. Pre-allocate and use `OsStr`/`OsString` for direct comparisons wherever possible.
