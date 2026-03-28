## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-05-18 - [String Parsing Overhead in Directory Traversal]
**Learning:** During directory traversal (`WalkDir`), converting each entry's file name to a string using `to_string_lossy()` inside the `filter_entry` closure introduces significant allocation and parsing overhead (~5x slower in benchmarks, e.g. 500ms vs 100ms for ~20,000 files).
**Action:** Pre-allocate and convert comparison strings (like `exclude_dirs`) into a `Vec<std::ffi::OsString>` outside the loop, and use direct `OsStr` comparisons for checking file names. Avoid `to_string_lossy()` in hot paths.
