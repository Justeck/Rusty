# Rusty Java 1.3.0-beta.1

Fabric and NeoForge counterparts for **Minecraft 26.3 / Java 25**. Horizon's native Rust 1.2.1 package remains the separate stable release.

- 42 configurable tools, Minecraft widgets, search/Enabled/Favorites/Recent, contextual help and recoverable menu access.
- Original block-texture context with adjustable retained area, typical/custom block targets, bounded received-chunk scanning, outlines and pointers.
- Clearable/rebindable shortcuts, reliable brief toggle taps, optional hold activation, independent UI/HUD size and 18 movable panels.
- Connection-scoped, server-authoritative opt-in for restricted multiplayer tools; local singleplayer grants. Ordinary information tools work on unmodified servers.
- World/dimension cache resets, ordinary-action ownership, explicit correction rearming and capped render/scan work.

**Install one matching loader jar.** Fabric requires Fabric API 0.165.2+26.3 and Loader 0.20.1+. NeoForge targets 26.3.0.101. Do not put these jars into Horizon's native mod installer.

[Java guide and permissions](https://github.com/Justeck/Rusty/blob/main/java/README.md) · [Validation and limitations](https://github.com/Justeck/Rusty/blob/main/docs/JAVA_VALIDATION.md)

This is a beta for one exact game version. The validation report separates real game checks from compilation and unverified cases. It does not claim feature parity with every third-party client, superiority in performance, or compatibility with arbitrary shader/rendering mods. Server anti-X-ray can withhold ore information; Rusty does not recover it.
