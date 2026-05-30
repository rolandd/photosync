## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.

## 2026-02-18 - Estimated Time of Arrival (ETA) for long-running batch operations
**Learning:** For bulk file operations (like syncing photos), users often want to know approximately when the task will complete, instead of just the elapsed time. Displaying ETA based on average speeds can significantly reduce user anxiety and provide better expectations.
**Action:** Include an ETA calculation dynamically based on rolling average processing durations in TUI/progress applications.
