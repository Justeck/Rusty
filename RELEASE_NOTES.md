# Rusty 1.2.7

**Arrange your HUD directly in the game view.**

- Click **Edit HUD** in Rusty’s menu header or a HUD feature’s settings. Coordinate sliders are replaced by direct manipulation of the actual overlays.
- Drag a panel to move it. Select it to reveal four corner handles; drag a handle to resize proportionally while keeping the opposite corner fixed.
- **Enter / Done** confirms and returns to the same menu. **Escape / Cancel** discards the draft. Default panel/layout changes are also provisional until confirmed.
- Enabled overlays keep their real appearance and feature states. Gameplay inputs remain blocked while editing; world simulation continues.
- Placement follows viewport changes. Moving a temporarily fitted panel preserves its saved size; panels remain accessible at screen edges and overlapping picks follow visible paint order.

Horizon-native Rust for **Horizon 38609b7 / API 409 / Minecraft 26.3**. Install `https://github.com/Justeck/Rusty/releases/tag/v1.2.7`, enable Rusty and complete a Performance build if requested. Press **Right Shift → Edit HUD**. Package: `rusty-1.2.7.hnmod` and `SHA256SUMS`.

[Validation and actual test scope](https://github.com/Justeck/Rusty/blob/main/docs/VALIDATION.md). Fabric and NeoForge counterparts: **1.3.0-beta.6**. Existing licenses and attribution remain intact.
