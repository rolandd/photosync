## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2023-11-20 - [Redundant String Allocation in Traversal]
**Learning:** Found that `file_walker` was allocating a new `String` via `e.file_name().to_string_lossy()` for every file encountered during directory traversal just to compare against the exclusion list. By pre-converting the exclusion list to `OsString` and comparing directly against the `OsStr` provided by `e.file_name()`, we eliminate a string allocation per file, which significantly speeds up traversal of large directories.
**Action:** Always prefer `OsStr` comparisons for file paths and names during hot loops rather than converting to `String` using `to_string_lossy()`.
