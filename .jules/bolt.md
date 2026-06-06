## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.
## 2026-05-18 - [Directory Traversal String Allocation]
**Learning:** During directory traversal with `WalkDir` in `src/pipeline.rs`, using `e.file_name().to_string_lossy()` inside the `filter_entry` closure caused unnecessary memory allocation and UTF-8 validation for every single file encountered.
**Action:** Convert exclusion rules to `OsString` upfront outside the iteration, and compare directly against `e.file_name()` (which is an `OsStr`). This avoids per-file allocations and relies on fast byte comparisons, leveraging OS-native representations.
