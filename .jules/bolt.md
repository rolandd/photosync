## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-23 - [Directory Traversal String Allocation]
**Learning:** Calling `to_string_lossy()` inside a hot directory traversal loop (like `walkdir`'s `filter_entry`) introduces a significant performance penalty due to repeated memory allocations and UTF-8 validation for every single file and directory visited.
**Action:** Always pre-convert exclusion lists or comparison targets to `OsString` or `&OsStr` outside the loop, and use direct `OsStr` comparisons within the traversal closure to achieve a ~5x speedup.
