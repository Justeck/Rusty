# Rusty 1.2 validation

Fresh checks identify the actual Rusty executable and source inputs. No historical game result is relabeled as a new test. [Build evidence](evidence/native-build.json), [game acceptance](evidence/game-acceptance.json), [named checks](evidence/game-checks.json), [launcher UI acceptance](evidence/launcher-ui.json) and [media provenance](evidence/media.json) keep their separate scopes.

| Gate | Result |
|---|---|
| Rust library/release-tool tests | 141 + 18 passed |
| Python tooling | 13 tooling tests passed; actual game-fixture behavior is covered by the fresh game checks below |
| Formatting and strict lint | Passed locally and in Linux CI |
| Native SDK compilation/package | Pinned SDK check and official package verification passed; Linux/Windows native archives byte-identical |
| Official Windows native build | Performance build completed successfully, waited to exit 0 |
| Core game behavior | 124 checks across 13 suites passed, including an owned official Java 26.3 server and a separate native Survival guest |
| UI/overlay behavior | 10 UI checks and 3 HUD-column checks passed, including clear/rebind, size endpoints, 18 independent HUD controls and actual process restart |
| Visual review | Original interface, help/binding, texture-context, profile and HUD frames reviewed at their captured scales |
| Short performance observation | 3 disabled + 9 enabled samples; stationary aggregation-cache reuse passed |
| Bounded stability | Ten-minute run completed; no panic/fatal marker observed; owned process stopped |
| Launcher build and actual GUI Play | Performance build through the official CLI; launcher 0.2.42 showed green Up to date, then GUI Play → title → Rusty settings → clean Quit. No Build button was offered for the already-built mod set. |
| Published release installation | Both repository and exact v1.2.0 release URLs installed successfully through launcher 0.2.42. All 115 payloads and archive bytes match the published/tested package. Supported cached Performance build exited 0; GUI Up to date → Play → title → clean Quit passed. |

[Release installation receipt](evidence/release-installation.json) records both accepted inputs, exact immutable tag/commit/asset, unchanged preferences and the executable shared by all game checks. [Published package](evidence/published-package.json), [release/workflow identities](evidence/published-release-api.json) and [official version check](evidence/official-version-check.json) are separate receipts.

## Exact native identity

- Supported public game: Minecraft 26.3, Horizon `6475f62f3c834b7489e1816bf47f47ea5eb2b879`, native API 359.
- Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.
- Accepted executable SHA-256: `fb177cd9216a567bb4982bfacedc47d0b07e6af6cdc43cfd2591050209ca29ea`.
- Staged input receipt SHA-256: `1c5d75b9a97e0f3f9e3ebf83de45e6d4a4aa41804ddf4b51391c83365d4f7646`.
- Native archive SHA-256: `7fb9ade0e05595fad6ddc019c0fdb0807a802f5667091df9eeb9a58ea908db0d`.
- Independent Linux [CI package receipt](evidence/ci-package.json) identifies its exact source/run/job/artifact; later documentation and test-observation changes leave the 115 native payloads unchanged.

## Corrected measurement and limits

One initial native multiplayer run timed out after a restored block. Its test had frozen the first zero-queue target count as its baseline. An unchanged-binary diagnostic passed all five lifecycle checks and captured a temporary 493→495 target transition during ordinary cache eviction/rebuilding. The first failing run did not record its exact baseline; it is not reconstructed or asserted. The stronger fixture waits across more than two scan intervals, distinct rendered frames and ready renderer queues, keeping exact original-minus-one/original counts, nearest targets, received block checks and host/guest source hashes. The failed raw receipt and diagnostic remain distinct from the corrected acceptance. [Settling observations](evidence/multiplayer-settling.json) record the corrected measurements.

A separate runner argument error was rejected before a performance game launched; the invocation was corrected without changing the fixture or executable. Acceptance records keep this separate from game behavior.

Owned server tests are separate from a player's public server. No personal world, authenticated account, public-server movement/building, inventory or combat was used in these fresh fixtures. Arbitrary server plugins, network latency, other hardware/backends and resource packs remain unverified. Servers can withhold or replace block data and retain authority over movement, reach, inventory and health. No anti-X-ray bypass, reference-client parity/superiority, leak-free or multi-hour claim is made.

UI 50% demonstrates contained compact layout; 200% keeps the outer menu/fixed controls inside the frame while detail content scrolls. Independent HUD placements are tested and visually reviewed in their recorded fixtures; not every possible user arrangement is validated. Screenshot tours are presentation media rather than continuous gameplay.

[Installation](INSTALLATION.md) · [Development](DEVELOPMENT.md) · [Performance](PERFORMANCE.md)
