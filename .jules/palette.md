## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.

## 2026-02-18 - TUI Consistent Component Status Colors
**Learning:** Hardcoding colors in individual TUI components (like progress bars or footers) can cause visual inconsistency when the application state changes to warning or error. Users may see conflicting signals (e.g. a red border but a green progress bar).
**Action:** Unify color semantics across the UI. Apply a dynamic status color function (e.g. `get_status_color`) to all relevant components (borders, gauges, footers) to ensure consistent, immediate feedback on application state.
