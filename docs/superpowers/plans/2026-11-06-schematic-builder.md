# Schematic Builder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Import, preview, build and export bounded schematics from Rusty's Build Planner with an actual file chooser and copyable materials.
**Architecture:** An independently implemented bounded Rust schematic codec/model feeds the native controller and its ordinary placement/inventory adapters. Java counterparts use their host NBT and actions with equivalent semantics. Existing box planning and safety controllers remain the foundation.
**Tech Stack:** Rust2024, flate2 matching the supported SDK, existing Horizon66f3981/API411, Java25/Fabric/NeoForge26.3.
**Spec:** docs/superpowers/specs/2026-11-06-schematic-builder.md

## Global Constraints

- Native1.4.0 / Java1.4.0-beta.1 for delivery; controller owns version/docs changes after implementation.
- No personal world/server writes. All live construction only disposable owned fixtures.
- Independent implementation; no reference-mod code/assets or removed legal notices.
- Input32MiB, expanded64MiB, depth64, cells1,048,576, axis256, palette65,536.
- Off/unbound, explicit start, manual/safety stops, no automatic resume; states-only world export and honest unsupported-data reporting.
- Parent handles releases/launcher/Modrinth and version files. Agents commit only their owned files, no broad `git add` or other-agent changes.

## Review Focus

- Host prediction is not acknowledgment: inventory swaps and all target writes require actual received settlement.
- Orientation/coupled writes cannot alter cells absent from the schematic or place a wrong state before detecting it.
- Malicious NBT, compression bombs, signed sizes, packed-long boundaries and conflicting regions fail within budgets.
- Dialog results/export scans are scoped and cannot reactivate after world change/cancel; unknown cells cannot become air.
- Existing user structures, settings and other automation must remain protected; complete means a fresh truthful verification.

### Task 1: Bounded Rust schematic model and codec

**Files:** Create `mixins/rusty/schematic.rs`, `tests/schematic.rs`, optional focused `schematic_nbt.rs`; modify `lib.rs`, Cargo.toml/Cargo.lock only for flate2. No UI/native manifests or Java edits.
**Interfaces:** `Schematic { name:String, size:[u32;3], palette:Vec<BlockState>, blocks:Vec<u32>, ignored_block_entities:u32, ignored_entities:u32 }`; `BlockState { name:String, properties:BTreeMap<String,String> }` with canonical `descriptor()`; `decode(&[u8])->Result<Schematic,String>`, `encode_sponge(&Schematic,data_version:i32)->Result<Vec<u8>,String>`, `Schematic::validate`, `transformed(rotation:u8,mirror:bool)->Result<Schematic,String>`, `block_totals()->BTreeMap<String,u32>`. Air cells remain dense; adapters map to registry/items. `pub const` limits shared. Types derive Clone/Debug/PartialEq. Access pure module as crate::rusty_schematic standalone and super::rusty_schematic in native game context. Coordinate transforms also update directional state properties (facing/axis/rotation/hinge/stair shape) or return explicit unsupported transform, never silently wrong. Preserve missing defaults for strict host resolution. No dependency on Horizon crates in this module.

- [ ] Write failing functional fixtures for Sponge2/3, vanillaNBT and packed `.litematic`4–7 including signed regions and words crossing64bits. Independent fixture writers, not decode/encode self-confirmation alone.
- [ ] Observe expected RED. Implement bounded NBT/gzip parsing, palette/cells/coordinate normalization, explicit ignored-data counters and Sponge3 export. Reference source is private at `../rusty-task/schematics/reference`; infer format behavior without copying implementation.
- [ ] Test malformed lengths/indexes/duplicate tags/conflicting regions/decompression limits and exact export-import states, non-square rotation/mirror properties and sorted totals.
- [ ] Run focused then all standalone Rust tests, fmt and strict clippy using CARGO_TARGET_DIR=target/standalone-msvc. Commit owned files and report exact RED/GREEN commands/results and API notes to task report.

### Task 2: Native chooser, export, materials and automatic construction

**Files:** Native `build_*`, new focused `schematic_io.rs`/`schematic_native.rs`/`schematic_agent.rs`, `inventory` receipt hooks, mining ownership/route helpers where required, catalog/clickgui/control/reset/tick and horizon-mixin.toml; focused native/core tests and game fixture tools. Do not touch Java or version/docs/release files. Coordinate any required Task1 API additions with controller.
**Consumes:** Task1 Schematic API; existing build_actions ordinary dry placement, inventory_actions swap, mining Grid safe_stand/can_walk and tagged landed Goto; SDK exact block registry/facts.item and clipboard.
**Produces:** Native menu file/load/clear/origin/rotate/mirror/materials-copy/start-stop/export-corner/save actions plus truthful progress/detail readouts; active/busy/stop and scoped reset helpers used for global mutual exclusion.

- [ ] Read `../rusty-task/schematics/native-platform-findings.md` first. Write and run reproducing tests for intended-state/all-write validation, no outside coupled write, safe no-mining route, acknowledged source/destination swap, transient scope/defaults and materials multipliers before production changes.
- [ ] Implement native Rust worker-thread Windows Explorer open/save adapter (Unicode/cancel/overwrite) and bounded atomic file IO; Linux/macOS best native fallback if available, clear errors otherwise. Keep main rendering thread responsive; chooser results carry scope/generation cancellation tokens.
- [ ] Implement received incremental states-only export using inclusive two-corner bounded selection and strict settled/readable cells, native clipboard list with registry item mapping, transformed exact-state preview, no auto-start.
- [ ] Implement explicit construction over ordinary movement/material sourcing/placement; validate every dry write against planned cells/state before send, wait actual acknowledgments, stop safety/denials, restore owned slot where safe, no demolition/scaffold/auto-resume. Support ordinary multi-material cubes, logs, stairs/slabs, doors/beds when normal action can match; other states report manual/unreachable rather than misplace.
- [ ] Run focused/all meaningful Rust checks and exact supported SDK type-check. Add actual owned native smoke fixture controls with no personal saves; parent executes final official builds and full game suites. Commit owned files/report tests and remaining exact limits.

### Task 3: Fabric and NeoForge schematic counterparts

**Files:** Only `java/**` source/core/tests/QA, excluding version/build/release metadata and docs owned by controller. Create focused schematic codec/model/dialog/controller classes rather than further bloating existing screen/mining classes. No Rust/native edits.
**Interfaces:** Equivalent spec semantics using host Java NBT with explicit decompressed/NBT budgets; data descriptors and fixture behavior match Task1; existing current-server capability policy remains authoritative.

- [ ] Add independent failing core codec/transform/material/safety tests. Implement modern formats and Sponge3 export, using ordinary source-format facts; no copied external mod implementation.
- [ ] Integrate native OS file picker on worker thread, generation/scoped load/export cancellation, Minecraft-style buttons, material readout/copy and two-corner received export. Preserve old box mode and direct placement.
- [ ] Implement ordinary full construction with bounded safe walking, hotbar/main inventory sourcing and server settlement, exact-state/coupled-cell placement checks, ownership and stops from spec. Registry item mapping and special material counts must match runtime block semantics. Permission view+movement+assistance; no ungranted restricted automation.
- [ ] Add actual owned QA cases: chooser import/export/clipboard hooks, multi-material walk+inventory sourcing, orientation/partials, denied/missing/manual/health/world stops and original regressions. Run core/build/package checks; compile QA. Do not launch games concurrently with parent's native/Java tests without scheduling. Commit owned files and task report.

### Task 4: Integrated game acceptance and delivery

**Files:** Parent-owned metadata/docs/workflows/evidence plus private test orchestration; implementation fixtures may be extended by a fresh scoped QA implementer if necessary.

- [ ] Read all task review findings, resolve Important/Critical with meaningful RED/GREEN, run whole affected checks and exact native SDK/package verification.
- [ ] Official supported Performance build, actual native owned SP/separate vanilla server construction/roundtrip safety plus existing regressions. Execute Java owned game checks sequentially for both loaders. Inspect every public screenshot before selection; actually test OS picker/cancel and clipboard in UI.
- [ ] Fresh whole-branch review, fix important findings and verify exact final code/binaries. Publish immutable native1.4.0 and Java1.4.0-beta.1 only on green source CI. Compare public source/package/classes, install public native through official launcher, real Build/Up-to-date/Play/title/Quit, prefs unchanged and one instance.
- [ ] Update professional GitHub/Modrinth guide, materials/import/export controls and reviewed real images; verify saved CDN downloads and all publication receipts. Remove superseded downloads/builds only after verified replacements and backups; preserve immutable source tags/required attribution.
