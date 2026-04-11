## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2024-05-18 - [Optimize walkdir path checking]
**Learning:** Checking excluded paths by converting `DirEntry::file_name()` via `.to_string_lossy()` on every entry during a large filesystem traversal imposes unnecessary UTF-8 validation and allocation overhead per iteration. This is particularly noticeable because WalkDir evaluates exclusions for *every* matched entry.
**Action:** Instead of converting every file name to a String to match an exclusion list, convert the exclusion list of Strings into `OsString` instances *once* outside the traversal loop. This allows `walkdir::DirEntry::file_name()` to perform direct OsStr comparisons (`name == ex`), entirely bypassing string allocation and encoding checks in the hot path.
