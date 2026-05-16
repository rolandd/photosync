## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.
## 2024-05-16 - [Eliminated hot loop PathBuf clones]
**Learning:** Found a performance bottleneck where `DestDirResult` (containing a `PathBuf`) was cloned from a cache on every single file processed in `src/pipeline.rs`, even on cache hits.
**Action:** Changed the caching strategy to update the `Option` and then yield a reference (`&dest_cache.as_ref().unwrap().2`) rather than cloning the value. This saves a heap allocation per file.
