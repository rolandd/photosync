## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.

## 2026-02-23 - Dynamic TUI Status Colors
**Learning:** Hardcoded colors in TUI components (like progress bars or footers) can create mixed signals when the application enters an error or warning state.
**Action:** Use a unified `get_status_color(app)` consistently across the interface (foregrounds, backgrounds, borders) to dynamically reflect the application's overall state.

## 2026-02-23 - Dynamic ETA for Long-Running Operations
**Learning:** For long-running batch operations in TUIs, users need a dynamically updating ETA based on recent throughput to reduce anxiety and provide better progress feedback.
**Action:** Compute and display an ETA by using the average duration of files in a rolling window multiplied by remaining items.
