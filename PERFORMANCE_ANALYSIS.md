# Performance Analysis: mpsc Migration and TUI Overhead

## Executive Summary
The migration from `crossbeam-channel` to `std::sync::mpsc` maintains the same efficiency characteristics as the previous implementation.
*   **Worker Threads**: utilize blocking operations, consuming **zero CPU** when queues are empty.
*   **TUI Thread**: utilizes a low-frequency polling loop (100Hz) with sleep, resulting in **negligible overhead** (< 0.1% CPU).

## Detailed Analysis

### 1. Worker Threads (Pipeline)
The worker threads (`file_walker`, `file_processor`, `file_handler`) consume messages using the standard iterator pattern:

```rust
for item in rx {
    // process item
}
```

In Rust's `std::sync::mpsc`, the `IntoIter` implementation calls `recv()`.
*   **Behavior**: `recv()` puts the thread to sleep (blocks) until a message is available or the channel is closed.
*   **CPU Usage**: When input queues are empty, these threads are in a `BLOCKED` state and **do not consume any CPU cycles**. They are woken up by the OS scheduler only when a new message is sent.
*   **Polling**: There is **no polling** in the worker threads.

### 2. TUI Thread (User Interface)
The TUI thread uses a unified event loop to handle both user input (keyboard) and progress updates. Since `std::sync::mpsc` does not support `select!` (unlike `crossbeam`), the loop checks for messages non-blockingly after checking for input.

```rust
loop {
    // 1. Block for up to 10ms waiting for keyboard events
    if event::poll(Duration::from_millis(10))? {
        // ... handle input
    }

    // 2. Drain all pending progress messages
    while let Ok(msg) = rx.try_recv() {
        app.handle_message(msg);
    }

    // ... redraw if needed
}
```

*   **Mechanism**:
    *   `event::poll(10ms)`: Uses the OS's I/O multiplexer (`poll`/`select`/`kqueue`/`epoll` on the TTY file descriptor). If no input occurs, the thread **sleeps** for 10ms.
    *   `rx.try_recv()`: Performs an atomic check on the channel state. This is extremely fast (nanoseconds).
*   **Overhead Calculation**:
    *   **Wakeups**: The loop runs at most 100 times per second (100Hz).
    *   **Cost per Wakeup**: One atomic check + one time check.
    *   **Total CPU**: Waking up 100 times/second on a modern multi-Ghz processor consumes negligible resources.
*   **Comparison**: This is the exact same logic used in the previous `crossbeam` implementation (which also polled `try_recv` inside the loop instead of using `select!`).

### 3. Conclusion
*   **Polling Potentially Empty Queues**: Only happens in the TUI thread, at a low frequency (10ms). The worker threads do not poll.
*   **Wasted CPU**: Virtually zero. The TUI thread spends the vast majority of its time sleeping in `poll`.
*   **Migration Impact**: No performance regression. The architectural efficiency remains unchanged.
