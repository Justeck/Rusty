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

Use stable Rust, the host's C/C++ linker/system dependencies, and Python 3.12+. Linux needs `libasound2-dev` and `libudev-dev`. The official Windows native lane uses the launcher's GNU toolchain. Keep pure MSVC and native GNU target caches separate. Prepared legal Minecraft resources are required for rendering/game checks and are not distributed.

The Python helper stages the exact source mixins into an isolated pinned host. `check` performs SDK/type checking; `build` uses Performance by default. `--profile dev` is for iteration. Optional prepared paths can avoid repeated downloads:

```sh
python tools/native.py check --horizon-tree /path/to/pinned/horizon --mixins-bin /path/to/horizon-mixins
python tools/native.py mesh-test --resources /path/to/prepared/26.3
```

The Rust tool writes deterministic `dist/rusty-1.2.0.hnmod` and `SHA256SUMS`; structural archive verification does not execute the game. Official SDK checking, compilation, launcher integration and live game behavior are separate gates.

## Actual game fixtures

Run one game fixture at a time, using a fresh output directory and explicit accepted executable/resources:

```sh
python tools/game_gui_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-gui
python tools/game_feature_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-features
python tools/game_ui12_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-ui
python tools/game_multiplayer_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-multiplayer
python tools/game_showcase_smoke.py --binary /path/to/horizon-client --resources /path/to/26.3 --output /path/to/new-showcase
```

Other `tools/game_*_smoke.py` fixtures cover inventory, ordinary assistance, automation, building, waypoint walking, awareness, HUD placement, chunk alignment, performance and endurance. Inspect each `--help`. Java-server tests require an official 26.3 server JAR and Java25; they use loopback and fresh fake-player data. Never point fixtures at an existing world or public server.

Every fixture creates isolated offline paths and stops owned processes. Record source, package, staged inputs, compiler/profile and executable hashes. Freeze native source during acceptance. Review original images separately from scripted assertions. A request being sent is not proof that a server accepted it. Keep unknown/partial observations explicit.

## Release gate

Push/PR checks run the pure/tooling lane, native SDK checks and package verification. Publishing is explicitly initiated only after real game/launcher acceptance. Preserve existing published tags and assets. A new source identity needs new game receipts and screenshots, not renamed historical results.
