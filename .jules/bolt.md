## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-18 - [String Allocation in Hot Loops]
**Learning:** `to_string_lossy()` in the `walkdir` filter closure caused measurable overhead due to allocations and UTF-8 validation for every file/directory scanned.
**Action:** Prefer `OsStr` direct comparison for filenames in traversal loops to avoid unnecessary string allocations.

## 2026-05-19 - [Eliminated Hot Loop PathBuf Clones]
**Learning:** `DestDirResult` (containing a `PathBuf`) was being cloned from `dest_cache` on every single file processed in `src/pipeline.rs`, even on cache hits.
**Action:** Update the cache entry in-place and yield a borrowed reference to the cached path, eliminating heap allocation and cloning overhead for sequential files.
