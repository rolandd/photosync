## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-19 - [String Allocation in File Traversal]
**Learning:** Calling `to_string_lossy()` on every entry during file system traversal creates a significant performance bottleneck due to repeated memory allocations and UTF-8 validation for each file and directory.
**Action:** Pre-allocate `OsString` vectors outside the traversal loop and perform direct `OsStr` comparisons for directory exclusions to avoid the overhead of string conversion, which operates ~5x faster.
