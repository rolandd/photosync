## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.
## 2024-08-08 - [String Conversion in Directory Traversal]
**Learning:** Calling `to_string_lossy()` on `OsStr` (like file names during directory traversal with `WalkDir`) inside high-throughput paths causes significant overhead due to repeated memory allocations and UTF-8 validation.
**Action:** Pre-allocate `OsString` vectors outside the loop and compare directly against raw `OsStr` references using `==` to avoid unnecessary allocations and string conversions.
