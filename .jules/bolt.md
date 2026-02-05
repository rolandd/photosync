## 2025-05-23 - [File Copying: Custom Loop vs BufWriter]
**Learning:** In this environment, wrapping `File` with `BufReader`/`BufWriter` (128KB) and using `std::io::copy` was ~7x slower than `std::io::copy` on raw files (8KB buffer). However, a custom copy loop with a reusable 128KB buffer provided a 14% speedup over `std::io::copy`.
**Action:** Avoid `BufReader`/`BufWriter` for bulk file copying. Use manual loops with large, reusable buffers.
