## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.
## 2026-02-18 - TUI Progress State Colors
**Learning:** Hardcoded colors in UI components (like progress bars or footers) miss an opportunity to convey global application state.
**Action:** Use dynamic styling functions (e.g., `get_status_color`) across related UI components to ensure consistent, global visual feedback of the app's current status.

## 2026-02-18 - TUI Estimated Time of Arrival (ETA)
**Learning:** Without an ETA, users cannot easily gauge if a long-running batch process will finish in seconds or hours, leading to poor UX.
**Action:** Add an ETA calculation based on the rolling average of recent operation durations multiplied by remaining work, and display it clearly alongside elapsed time.
