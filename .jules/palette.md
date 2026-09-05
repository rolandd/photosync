## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Elapsed Time Indicator
**Learning:** For long-running batch operations, users need a sense of temporal scale to estimate completion or detect hangs.
**Action:** Display a wall-clock timer (MM:SS) that starts immediately and freezes upon completion.
## 2026-02-18 - Estimated Time of Arrival (ETA)
**Learning:** For long-running operations in TUI, users need real-time estimations (ETA) to manage expectations and reduce anxiety, particularly on batch jobs where just speed (MB/s) forces mental math.
**Action:** When designing or updating progress indicators for batch processes, always incorporate an automated, dynamically updating ETA metric based on recent operation throughput.
