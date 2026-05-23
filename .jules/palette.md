## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.
## 2024-05-18 - Improve TUI Context and Experience
**Learning:** Found that TUI feedback via color coding the progress gauge foreground and footer background gives clear context of success and errors as operations complete. ETA provides helpful feedback in asynchronous TUI environments where wait times can be significant.
**Action:** When designing or adding to TUI layouts, utilize state-driven dynamic colorization for prominent UI elements (gauge foregrounds, borders, active footer texts) and provide explicit progress estimators (like ETA based on processing windows) instead of simple counters.
