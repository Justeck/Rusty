![Rusty - integrated copper R wordmark](https://raw.githubusercontent.com/Justeck/Rusty/v1.2.3/docs/assets/rusty-wordmark.png)

# Rusty 1.2.3

**X-ray now changes terrain only.** It no longer implicitly draws target tracer lines, target boxes or Finder target readouts. Enable **Finder** separately for those overlays; **V** is its native default shortcut. Turning X-ray on/off preserves Finder and saved line preferences, so deliberately enabled Finder overlays keep working with ordinary terrain too.

The X-Ray panel contains terrain, activation and target-selection controls. Line/box controls live under Finder. Original-texture terrain frames and the 25% default texture coverage remain part of X-ray.

Targets **Horizon Nightly 38609b7 / API 409 / Minecraft 26.3**. Install `https://github.com/Justeck/Rusty/releases/tag/v1.2.3` in Horizon's native GitHub input, enable Rusty, choose Performance build if offered, wait for Up to date, then Play. Right Shift opens the menu. Package: `rusty-1.2.3.hnmod`, with `SHA256SUMS`.

Fresh build and rendered-regression scope is recorded in [Validation](https://github.com/Justeck/Rusty/blob/main/docs/VALIDATION.md). The separate Fabric/NeoForge patch is Java 1.3.0-beta.2; Java jars do not go into Horizon's native installer.

Only received data is used. No server anti-X-ray bypass, reach extension or override of server authority is provided. Apache-2.0 with applicable MIT attribution and notices preserved.
