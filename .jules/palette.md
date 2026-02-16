## 2026-02-17 - TUI Persistent Keyboard Hints
**Learning:** In TUI applications, users often forget available keybindings if they aren't visible.
**Action:** Always display critical navigation keys (like 'q' to quit) on the screen border using `title_bottom`.

## 2026-02-18 - TUI Status via Border Color
**Learning:** Changing the main container's border color based on success/error state (Red/Green) provides immediate, glanceable feedback without requiring extra text.
**Action:** Use `Block::border_style` dynamically based on application state in TUI apps.

## 2026-02-18 - Immediate TUI Status Feedback
**Learning:** Users need to know about errors/warnings immediately during long-running processes, not just at the end. Changing the progress bar color dynamically (Red for errors) provides this instant feedback.
**Action:** Update status indicators (colors, text) in real-time based on error counts, rather than waiting for the final "Done" state.
