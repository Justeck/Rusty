# Rusty Java 1.3.0-beta.6

**Move and resize enabled HUD overlays in the clear game view.**

Open **Right Shift → Edit HUD**, click a panel, drag its body to move it, or drag a corner handle to resize proportionally. **Enter / Done** saves the draft and returns to the same menu. **Escape / Cancel** discards changes. Default panel and Default layout operate on the draft. Opening the editor never enables a feature, and the menu shortcut cannot replace an unfinished editor.

World rendering remains clear while editing, without changing your video settings. HUD sizing stays separate from menu sizing. Existing saved placements remain usable; panels fit the available viewport and resizing stops at the edge while preserving the opposite corner. Overlapping panels can be placed intentionally.

Fabric and NeoForge for **Minecraft 26.3 / Java 25**. Install one matching loader jar. Fabric requires Loader 0.20.1+ and Fabric API 0.165.2+26.3; NeoForge targets 26.3.0.101. Existing multiplayer permission policy remains unchanged. Tests use owned offline worlds; no fresh public-server acceptance is claimed.

[Validation](https://github.com/Justeck/Rusty/blob/main/docs/JAVA_VALIDATION.md) · [Horizon-native edition](https://github.com/Justeck/Rusty/releases/tag/v1.2.7).
