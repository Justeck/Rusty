# Rusty 1.2 validation

This page records fresh checks of the Rusty source identity. No historical raw game result has been relabeled as a new Rusty test.

| Gate | Current result |
|---|---|
| Rust pure/tool tests | 141 library +18 native release-tool tests passed after source rename |
| Python tooling | 13 tests passed after source rename |
| Formatting/strict lint | In progress |
| Native SDK compilation/package | In progress |
| Actual launcher Build/Play | Pending the Rusty-native build |
| Singleplayer/owned multiplayer | Pending fresh executable fixtures |
| Original screenshots/tour | Pending fresh Rusty captures |
| Public installable release | Not yet published |

Exact source/package/executable identities and raw receipt hashes will be recorded as each gate completes. Compilation alone does not establish multiplayer behavior. Owned Java/native server fixtures are separate from a player's public server. No public-server movement, building, inventory or combat test is authorized as part of branding work.

The supported target is Minecraft26.3, Horizon6475f62/nativeAPI359 and mixins e8cd0aa38cffef49cb2e088ef716baa107148c02. Server anti-X-ray, server authority, unknown chunks, finite scan budgets, other hardware and long sessions remain explicit limitations. Reference-client parity and perfection are not claimed.

[Installation](INSTALLATION.md) · [Development](DEVELOPMENT.md) · [Performance](PERFORMANCE.md)
