## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.

## 2026-02-19 - ETA Indicator for Batch Progress
**Learning:** In addition to elapsed time, users benefit from an Estimated Time of Arrival (ETA) to anticipate completion time during long file copy operations.
**Action:** Compute ETA using a rolling average of recent copy durations multiplied by remaining files (`files_with_exif - exif_processed`), showing it once the scan is complete and copy durations are established.
