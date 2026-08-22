## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.

## 2026-02-23 - Dynamic Progress Feedback (ETA)
**Learning:** For batch operations involving a large number of files, displaying simple progress bars (percent complete) leaves users anxious about how much longer the operation will take. Users feel more in control when provided an Estimated Time of Arrival (ETA).
**Action:** Always compute and show a dynamically updating ETA (based on recent throughput) in TUIs for long-running batch processing tasks.
