## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.
## 2026-02-18 - TUI Estimated Time of Arrival (ETA)
**Learning:** Providing an Estimated Time of Arrival (ETA) for large operations greatly reduces user anxiety. Calculating an ETA based on the average duration of the last `N` operations offers a responsive estimate that dynamically adjusts to current system conditions.
**Action:** When implementing ETA calculations in TUI batch jobs, utilize a rolling window of recent operation timings combined with the number of remaining items to project completion time.
