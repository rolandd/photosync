## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.
## 2026-05-30 - [Overhead of to_string_lossy in traversal]
**Learning:** Calling `to_string_lossy()` on `OsStr` for directory exclusion matching during traversal (`WalkDir`) introduces substantial overhead due to repeated UTF-8 validation and potential string allocations. Profiling showed it to be ~2x slower than direct `OsStr` byte comparisons on happy paths, and much worse if allocations actually occur.
**Action:** When filtering filesystem paths in tight loops or walkers, pre-allocate exclusions as `std::ffi::OsString` and use direct `OsStr` equality checks (`name == ex.as_os_str()`) instead of converting names to Strings.
