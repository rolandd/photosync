## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-24 - [OsStr Allocation Overhead]
**Learning:** In the hot path of file traversal (`file_walker`), calling `to_string_lossy()` on every `DirEntry` filename for exclusion checks adds measurable overhead due to UTF-8 validation and potential allocation.
**Action:** Pre-convert exclusion strings to `OsString` and perform direct `OsStr` comparisons (`name == ex.as_os_str()`) inside the traversal loop to avoid this overhead.
