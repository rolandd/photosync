# Report: Migration from `crossbeam-channel` to `std::sync::mpsc`

## Overview
This report evaluates the feasibility and steps required to remove the `crossbeam-channel` dependency from the `photosync` codebase and replace it with the standard library's `std::sync::mpsc`.

## Current Usage
The `crossbeam-channel` crate is currently used in the following files:

1.  **`src/pipeline.rs`**:
    *   Uses `Sender` and `Receiver` in function signatures (`file_walker`, `file_processor`, `file_handler`).
    *   Iterates over `Receiver` (`for item in rx`).
    *   Sends messages using `tx.send(item)`.

2.  **`src/main.rs`**:
    *   Uses `bounded(size)` to create channels.
    *   Clones `Sender`s (`progress_tx`) for multiple producer threads (walker, processor, handler).
    *   Spawns threads passing these senders and receivers.

3.  **`src/tui.rs`**:
    *   Uses `Receiver` in `run_tui`.
    *   Polls the receiver using `rx.try_recv()`.

## Feasibility Analysis
The usage pattern in `photosync` is primarily **Multi-Producer, Single-Consumer (MPSC)**:
*   Pipeline stages (`walker` -> `processor` -> `handler`) are 1:1 (Single Producer, Single Consumer), which is a subset of MPSC.
*   Progress reporting (`walker`, `processor`, `handler` -> `main/TUI`) is M:1 (Multi-Producer, Single Consumer).

The standard library's `std::sync::mpsc` (Multi-Producer, Single-Consumer) is perfectly suited for this architecture.

### Key Differences & Adaptations
*   **Channel Creation**: `crossbeam::bounded` corresponds to `std::sync::mpsc::sync_channel`.
*   **Sender Types**:
    *   `crossbeam::Sender` is used for both bounded and unbounded channels.
    *   `std::sync::mpsc` distinguishes between `Sender` (unbounded, async) and `SyncSender` (bounded, sync).
    *   Since `photosync` uses bounded channels (`CHANNEL_BUFFER_SIZE = 1024`), we must use `SyncSender`.
*   **Receiver Types**: Both libraries provide a `Receiver` that implements `Iterator` and has `try_recv()`.
*   **Cloning**: `std::sync::mpsc::SyncSender` is `Clone` and `Send`, allowing it to be shared across threads, just like `crossbeam::Sender`.
*   **Performance**: While `crossbeam` is historically faster, for the granularity of file system operations and EXIF parsing, the overhead of `std` channels is negligible.

## Migration Plan
1.  **Update `src/pipeline.rs`**:
    *   Replace `crossbeam_channel::{Sender, Receiver}` with `std::sync::mpsc::{SyncSender, Receiver}`.
    *   Update function signatures to accept `SyncSender`.

2.  **Update `src/main.rs`**:
    *   Replace `crossbeam_channel::bounded` with `std::sync::mpsc::sync_channel`.
    *   Ensure type inference or explicit types match `SyncSender`.

3.  **Update `src/tui.rs`**:
    *   Replace `crossbeam_channel::Receiver` with `std::sync::mpsc::Receiver`.
    *   `try_recv` usage remains the same.

4.  **Update `Cargo.toml`**:
    *   Remove `crossbeam-channel` dependency.

## Conclusion
Removing `crossbeam-channel` is a low-risk, high-feasibility refactoring. It reduces the dependency footprint without requiring significant architectural changes or loss of functionality.
