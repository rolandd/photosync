## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.

## 2026-03-24 - Consistent Status Colors in TUI Components
**Learning:** Hardcoded success colors (like `Color::Green` for "Complete" and progress bars) create confusing UX when an operation finishes with warnings or errors. TUI components must consistently reflect the overall application state to provide clear feedback.
**Action:** Always use a centralized status color function (e.g., `get_status_color(app)`) for all state-indicating UI elements (borders, progress bars, text) rather than hardcoding default success colors.
