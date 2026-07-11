## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.
## 2026-05-19 - [Avoid `to_string_lossy()` in hot loops]
**Learning:** File system traversal in `src/pipeline.rs` (`file_walker`) is optimized to perform direct `OsStr` comparisons for directory exclusions. Pre-allocating `OsString` vectors outside the loop avoids the allocation and UTF-8 validation overhead of calling `to_string_lossy()` on every entry, operating ~5x faster.
**Action:** Avoid using `to_string_lossy()` inside hot loops like file traversal. Pre-allocate and map strings into `OsString` for comparisons instead.
