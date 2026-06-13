## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2024-06-13 - [Directory Traversal String Allocation]
**Learning:** During recursive file traversal with `walkdir`, converting `OsStr` to a `String` (e.g., via `to_string_lossy()`) inside the hot loop for exclusion filtering introduces significant allocation and UTF-8 validation overhead for every file and directory.
**Action:** When filtering paths based on string lists during a high-frequency loop like directory traversal, always pre-convert the filter strings to `std::ffi::OsString` *outside* the loop. Then, directly compare these to `std::ffi::OsStr` pointers inside the loop. This provides zero-allocation comparisons.
