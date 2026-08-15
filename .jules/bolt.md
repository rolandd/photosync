## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2023-11-09 - [WalkDir string allocation overhead]
**Learning:** Calling `to_string_lossy()` on `OsStr` (e.g. from `e.file_name()`) inside a high-throughput traversal loop like `WalkDir::filter_entry` introduces significant overhead due to memory allocation and UTF-8 validation on every single file check.
**Action:** When filtering paths based on string exclusions during directory traversal, always pre-allocate the exclusion list as `OsString` (or direct byte representations if applicable) outside the loop and compare directly against raw `OsStr` references.
