## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-18 - [Avoid `to_string_lossy` in Hot Paths]
**Learning:** Calling `to_string_lossy()` on `OsStr` (e.g., `e.file_name().to_string_lossy()`) during file system traversal (like in `WalkDir` loops) introduces significant overhead due to memory allocation and UTF-8 validation per file.
**Action:** When filtering or comparing file names from the OS, convert your reference strings into `std::ffi::OsString` once outside the loop, and use direct equality comparisons on the raw `OsStr` references inside the hot path.
