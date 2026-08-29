## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.
## 2024-05-14 - TUI ETA Calculation
**Learning:** For long-running batch operations in TUIs, always compute and display a dynamically updating ETA (Estimated Time of Arrival) based on recent throughput to reduce user anxiety and provide better progress feedback. This insight comes from the memory instructions.
**Action:** Always include ETA calculations in TUI progress displays where operations take noticeable time.
