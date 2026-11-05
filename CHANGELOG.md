# Changelog

## Rusty 1.2.2

- Updated the native host pin to Horizon Nightly 38609b7, native API 409, Minecraft 26.3.
- Fixed Freecam compilation against Horizon's device-aware HeldKeys input container. Clear only movement mappings while detached, and restore only physically held movement keys on exit.
- Updated exact-revision validation for the upstream renderer test fixture; no upstream production changes.
- Fresh launcher Performance build, Play and bounded game checks are recorded separately from earlier releases in [Validation](docs/VALIDATION.md).

## Rusty 1.2.1

- Integrated wordmark: the copper badge is the R in Rusty, followed only by USTY.
- Standalone badge in the native and Java menu headers, with clear spacing.
- Native menu uploads one small image when opening, avoiding per-frame texture uploads.
- Updated repository cover and artwork provenance.


## Rusty 1.2.0

- Unified native Rust toolkit with 42 actual modules and a Minecraft-style interface.
- Original copper/graphite R emblem and RUSTY wordmark with restrained patina detail.
- Consistent native ID, commands, configuration keys, source paths and release-tool naming.
- Live Enabled collection, contextual help, shortcut rebinding/clearing, and independent menu size.
- Independent position/size/reset controls for 18 HUD overlays and content-aware default columns.
- Original block-texture context, default 25% coverage, X toggle and optional Hold activation.
- Exact Horizon 6475f62 / native API 359 target, deterministic native packaging and automated checks.
- Fresh build, game and launcher acceptance are recorded in [Validation](docs/VALIDATION.md).
