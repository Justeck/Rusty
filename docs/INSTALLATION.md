# Install Rusty

Rusty 1.4.0 is a Horizon-native Rust source mod for Minecraft Java 26.3. Horizon compiles the client with the enabled native mods. Fabric, Forge and NeoForge are not runtime dependencies.

1. Select the original **Horizon** instance and open **Mods**.
2. Choose **Add a Native Mod from GitHub** and paste `https://github.com/Justeck/Rusty`.
3. Review the resolved immutable revision and native-source notice, then enable Rusty.
4. Use the supported Horizon revision **66f3981**. If Horizon shows **Needs rebuild**, select **Performance build**, then **Rebuild on Horizon Nightly 2026-11-05** (or **Build** if already on that base).
5. Wait for **Up to date**, then press **Play**. An identical mod set can reuse its verified native build and show green immediately.
6. In a world, press **Right Shift**. **Options → Mods → Rusty** is the fallback.

The versioned package is `rusty-1.4.0.hnmod`, accompanied by `SHA256SUMS`. A native source package still requires Horizon's build. A full source ZIP, if supplied, is for development/review; install the native package or GitHub source URL.

The repository URL follows the latest public release when available; Horizon resolves it to an immutable revision before installation. A fixed commit/release URL also works. Review an offered update's source and host pin before replacing a working installation. Do not combine native mods built for incompatible Horizon revisions.

For the fixed 1.4.0 release, paste `https://github.com/Justeck/Rusty/releases/tag/v1.4.0`. Horizon resolves a release input to its immutable native asset. [Validation](VALIDATION.md) separates exact-source compilation, actual disposable-world/server checks and published-release installation. Earlier receipts retain their historical scope.

## Compatibility

| Component | Supported target |
|---|---|
| Minecraft Java | 26.3 |
| Horizon | `66f3981f522b24c250b21523a70080965b940424` |
| Native API | 411 |
| Mixins | `e8cd0aa38cffef49cb2e088ef716baa107148c02` |

## First use

**X** toggles X-ray; optional Hold mode is in its settings. Change block-texture coverage to retain more or less terrain context. Use Advanced / Profiles for detailed block IDs, states, height filters, colors and presets. The Enabled collection shows active modules. Use Clear to unbind a shortcut; recover through native settings if the menu key is unbound.

**Edit HUD** opens the game view for dragging and corner-handle resizing. **Enter / Done** confirms and returns to the mod menu; **Escape / Cancel** discards the draft. HUD size stays independent of menu size. Layout preferences persist locally and remain independent from feature profiles. Status HUD is enabled by default; other modules are opt-in.

**Auto Mine** is off and unbound by default. Choose a resource and block goal, put suitable tools in the hotbar, then start it explicitly. Manual input or opening a menu stops the mode; it never resumes itself. [Controls and conservative safety limits](AUTO_MINE.md).

**Area Clear** is also off and unbound. Aim at opposite corners and use its **Set first corner / Set second corner** buttons, review the box and explicitly enable it. It shares Auto Mine's exclusive normal-control owner and stops instead of excavating unsafe or protected leftovers. [Area Clear guide](AREA_CLEAR.md).

## Troubleshooting

| Symptom | Check |
|---|---|
| Build fails | Compare the enabled native mods' exact host pins; preserve the first useful error. |
| Rusty is missing | Confirm the enabled native set was built and that Play uses its `native-…` version. |
| A shortcut does nothing | Check native Keys settings, conflicts, menus/focus, and whether the module is enabled. |
| Missing ores | Check targets, height/radius/state filters and partial results; server anti-X-ray may withhold or replace data. |
| Textures look ordinary | The maximum coverage endpoint restores ordinary terrain and lighting. |
| Information is unavailable | The server must supply the relevant state; unknown data is not invented. |

[Usage](USAGE.md) · [Modules](MODULES.md) · [Development](DEVELOPMENT.md) · [Validation](VALIDATION.md)
