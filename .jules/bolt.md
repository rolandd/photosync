## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2024-04-25 - Avoid String allocation and UTF-8 validation in hot loops
**Learning:** Calling `to_string_lossy()` on `OsStr` (e.g. from `e.file_name()` in a `WalkDir` loop) forces UTF-8 validation and potential heap allocation for every entry traversed. This significantly slows down filesystem traversal on large directories.
**Action:** Pre-convert list of expected exclusions to `OsString` before the loop, and use direct `OsStr` vs `OsString` comparison during the directory walk.
