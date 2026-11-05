# Install Rusty

Rusty 1.2.0 is a Horizon-native Rust source mod for Minecraft Java 26.3. Horizon compiles the client with the enabled native mods. Fabric, Forge and NeoForge are not runtime dependencies.

1. Select the original **Horizon** instance and open **Mods**.
2. Choose **Add a Native Mod from GitHub** and paste `https://github.com/Justeck/Rusty`.
3. Review the resolved immutable revision and native-source notice, then enable Rusty.
4. Use the supported Horizon revision **6475f62**. If Horizon shows **Needs rebuild**, choose **Build → Performance**.
5. Wait for **Up to date**, then press **Play**. An identical mod set can reuse its verified native build and show green immediately.
6. In a world, press **Right Shift**. **Options → Mods → Rusty** is the fallback.

The versioned package is `rusty-1.2.0.hnmod`, accompanied by `SHA256SUMS`. A native source package still requires Horizon's build. A full source ZIP, if supplied, is for development/review; install the native package or GitHub source URL.

The repository URL follows the latest public release when available; Horizon resolves it to an immutable revision before installation. A fixed commit/release URL also works. Review an offered update's source and host pin before replacing a working installation. Do not combine native mods built for incompatible Horizon revisions.

For the fixed 1.2.0 release, paste `https://github.com/Justeck/Rusty/releases/tag/v1.2.1`. Both this release URL and the repository URL were accepted by the official launcher installer and resolved to the same immutable commit and native asset. The installed release was then built/verified and opened through the launcher Play button. [Installation evidence](evidence/release-installation.json).

## Compatibility

| Component | Supported target |
|---|---|
| Minecraft Java | 26.3 |
| Horizon | `6475f62f3c834b7489e1816bf47f47ea5eb2b879` |
| Native API | 359 |
| Mixins | `e8cd0aa38cffef49cb2e088ef716baa107148c02` |

## First use

**X** toggles X-ray; optional Hold mode is in its settings. Change block-texture coverage to retain more or less terrain context. Use Advanced / Profiles for detailed block IDs, states, height filters, colors and presets. The Enabled collection shows active modules. Use Clear to unbind a shortcut; recover through native settings if the menu key is unbound.

HUD position/size controls are independent of menu size. Layout preferences persist locally and remain independent from feature profiles. Status HUD is enabled by default; other modules are opt-in.

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
