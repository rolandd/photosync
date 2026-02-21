## 2026-05-18 - [Buffered I/O Regression]
**Learning:** Adding `BufReader`/`BufWriter` (128KB) to `std::io::copy` caused a ~17% performance regression compared to bare `File` handles. `std::io::copy` and `std::fs::copy` likely leverage kernel-side optimizations like `copy_file_range` or `splice` on Linux, which userspace buffering interferes with.
**Action:** Avoid wrapping `File` handles in `BufReader`/`BufWriter` for file-to-file copies unless there is a specific need for userspace processing. Prefer `std::fs::copy` or bare `std::io::copy`.

## 2026-02-21 - [Reuse Open File Handles]
**Learning:** `nom-exif` supports `MediaSource::seekable(&mut File)`, allowing file handles to be reused for parsing and then processing. This eliminates redundant `open` syscalls in the hot path.
**Action:** When optimizing file pipelines, check if libraries support `Read + Seek` traits on open files instead of paths to avoid double opens.
