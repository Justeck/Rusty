# Build and test Rusty

The runtime and optional native packaging tool are Rust. Python is development-only orchestration for actual game fixtures. Exact native pins are in `horizon-native.json`, `compatibility.json` and `mixins/rusty/horizon-mixin.toml`.

```sh
cargo fmt --check
cargo test --locked --features native-tools
cargo clippy --locked --all-targets --features native-tools -- -D warnings
python -m unittest discover -s tools -p 'test_*.py'
cargo run --locked --features native-tools --bin rusty-native -- check
python tools/native.py check
python tools/native.py build
cargo run --locked --features native-tools --bin rusty-native -- package
python tools/native.py verify-package
```

The current native target is Horizon 66f3981 / API 411. Use stable Rust, the host's C/C++ linker/system dependencies, and Python 3.12+. Linux needs `libasound2-dev` and `libudev-dev`. The official Windows native lane uses the launcher's GNU toolchain. Keep pure MSVC and native GNU target caches separate. Prepared legal Minecraft resources are required for rendering/game checks and are not distributed.

The Python helper stages the exact source mixins into an isolated pinned host. `check` performs SDK/type checking; `build` uses Performance by default. `--profile dev` is for iteration. Optional prepared paths can avoid repeated downloads:

```sh
python tools/native.py check --horizon-tree /path/to/pinned/horizon --mixins-bin /path/to/horizon-mixins
python tools/native.py mesh-test --resources /path/to/prepared/26.3
```

The Rust tool writes deterministic `dist/rusty-1.2.2.hnmod` and `SHA256SUMS`; structural archive verification does not execute the game. Official SDK checking, compilation, launcher integration and live game behavior are separate gates.

## Actual game fixtures

Run one game fixture at a time, using a fresh output directory and explicit accepted executable/resources:

```sh
python tools/game_gui_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-gui
python tools/game_feature_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-features
python tools/game_ui12_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-ui
python tools/game_multiplayer_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-multiplayer
python tools/game_showcase_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-showcase
```

The performance and endurance observers additionally require `psutil==7.2.2`, the version used by the recorded Windows run: `python -m pip install psutil==7.2.2`. This is a development-only process-observation dependency; the native mod has no Python runtime dependency.

Other `tools/game_*_smoke.py` fixtures cover inventory, ordinary assistance, automation, building, waypoint walking, awareness, HUD placement, chunk alignment, performance and endurance. Inspect each `--help`. Java-server tests require an official 26.3 server JAR and Java25; they use loopback and fresh fake-player data. Never point fixtures at an existing world or public server.

Every fixture creates isolated offline paths and stops owned processes. Record source, package, staged inputs, compiler/profile and executable hashes. Freeze native source during acceptance. Review original images separately from scripted assertions. A request being sent is not proof that a server accepted it. Keep unknown/partial observations explicit.

## Release gate

Push/PR checks run the pure/tooling lane, native SDK checks and package verification. Publishing is explicitly initiated only after real game/launcher acceptance. Preserve existing published tags and assets. A new source identity needs new game receipts and screenshots, not renamed historical results.

For the native menu logo, run the executable from its official launcher-installed folder with `native-mods.json` beside it. Horizon allocates the native image atlas slot only when that build metadata lists native mods. Running a bare target executable lacks that slot and is not a valid logo integration check.

## Publishing accepted versions

After the exact runtime source passes the supported build and applicable isolated game checks, merge an explicitly named **[release]** commit to `main`, or use the existing manual workflow dispatch. The native and Java publishers run only after their respective build/check jobs succeed. The native publisher keeps GitHub latest; Java remains a prerelease. Already published versions and tags are preserved. Bump the relevant version identities before a new release and retain exact package/game receipts. A release marker authorizes publishing; it does not substitute for real game acceptance.

## Schematic integration

The native codec and construction policy remain Rust. `mixins/rusty/platform` is a dependency-free native helper crate for the operating system file picker and failure-safe replacement. Its narrow safe interface isolates the required Windows calls; the injected game code retains Horizon's prohibition on unsafe code. The source packager follows the bundle's declared crate and includes its manifest and source, with build outputs excluded.

`tests/schematic.rs` exercises independent NBT fixtures, gzip integrity, bounds and exact transformations. `tests/schematic_build.rs` covers walking and placement policy, material accounting and inventory processing. `tests/schematic_io.rs` exercises bounded Unicode file handling and cancellation. These checks do not establish live construction.

Use `tools/game_schematic_smoke.py` with the exact candidate executable, prepared resources and a **new** output directory for disposable native survival construction. Its cases keep a twelve-cell, ten-item multi-material fixture, far targets requiring walking and full-hotbar main-inventory sourcing. Separate-server acceptance and operating-system picker/clipboard controls remain distinct checks. The script never opens a personal save. Java focused acceptance is selected with `RUSTY_QA_SCHEMATICS=1` and independently generated files supplied through `RUSTY_QA_SCHEMATIC_FIXTURES`; unset both for the original regression suite.
