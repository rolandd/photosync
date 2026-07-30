## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.

## 2026-03-05 - Consistent TUI Status Colors
**Learning:** Hardcoding colors in TUI components (e.g., Green for completion prompts or Cyan for progress bars) can conflict with the actual application state, leading to inconsistent user experience. For example, showing a green "completion" prompt even when critical errors occurred sends mixed signals to the user.
**Action:** Use a centralized status color source of truth (like a `get_status_color` function) dynamically for all relevant UI components (borders, gauges, footers) to ensure consistent visual feedback reflecting the true application state.
