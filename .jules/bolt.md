## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-18 - [File Walk Allocation Overhead]
**Learning:** Using `to_string_lossy()` inside hot paths like directory traversal (e.g., in `WalkDir` `filter_entry` closures) introduces significant performance overhead due to repeated memory allocations and UTF-8 validation per directory entry.
**Action:** Always prefer direct comparisons against `OsStr` or pre-allocated `OsString` vectors when dealing with file paths in high-throughput functions. Convert comparison strings to `OsString` once outside the loop.
