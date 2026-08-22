## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-18 - [String Allocation in Hot Loops]
**Learning:** `to_string_lossy()` in the `walkdir` filter closure caused measurable overhead due to allocations and UTF-8 validation for every file/directory scanned.
**Action:** Prefer `OsStr` direct comparison for filenames in traversal loops to avoid unnecessary string allocations.

## 2026-05-19 - [Eliminated Hot Loop PathBuf Clones]
**Learning:** `DestDirResult` (containing a `PathBuf`) was being cloned from `dest_cache` on every single file processed in `src/pipeline.rs`, even on cache hits.
**Action:** Update the cache entry in-place and yield a borrowed reference to the cached path, eliminating heap allocation and cloning overhead for sequential files.

## 2026-08-22 - [Optimizing Path Sanitization Hot Path]
**Learning:** Checking for Windows reserved filenames (`CON`, `PRN`, etc.) and invalid characters in a tight loop using `to_ascii_uppercase()` and `s.chars().any()` caused heavy allocation and UTF-8 decoding overhead. A benchmark showed that checking string length/`eq_ignore_ascii_case()` and using `s.bytes().any()` (for ASCII checks < 128) reduced execution time by up to ~75%.
**Action:** When performing simple character matching or reserved word checks on paths in hot loops, use `.bytes()` if the target characters are guaranteed ASCII, and avoid case-conversion allocations by using `eq_ignore_ascii_case()` or exact match branching.

## 2026-08-22 - [Reverted Needs Char Replacement Optimization]
**Learning:** Trying to optimize `needs_char_replacement` by checking for ASCII control chars `< 32` and `127` using `.bytes()` iterates caused a bug. While it sped up the loop on strings, it incorrectly created false negatives for unicode control characters since they evaluate to bytes `>= 128` and skipped the `needs_char_replacement` check.
**Action:** Always maintain correctness. If iterating over `.chars()` is needed for correctness on UTF-8 strings for methods like `is_control()`, do not swap it to `.bytes()` just for performance without realizing the functional regression on unicode paths.
