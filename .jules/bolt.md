## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-18 - [String Allocation in Hot Loops]
**Learning:** `to_string_lossy()` in `walkdir` filter closure caused measurable overhead (~5-6%) due to allocations for every file scanned, especially on large datasets.
**Action:** Prefer `OsStr` direct comparison for file names in tight loops or filters to avoid unnecessary allocations and UTF-8 validation.
