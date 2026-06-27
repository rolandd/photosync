## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.
## 2026-06-27 - [WalkDir String Allocation Overhead]
**Learning:** Calling `.to_string_lossy()` inside `WalkDir`'s `filter_entry` closure causes a string allocation and UTF-8 validation on every single file system entry encountered during directory traversal. This becomes a significant bottleneck when scanning large SD cards.
**Action:** When filtering paths or filenames from an `OsStr` (like those returned by `DirEntry::file_name()`), prefer converting the comparison strings to `OsString` ahead of time instead of converting the `OsStr` to a `String` on each iteration. This allows direct comparison without allocation or validation overhead.
