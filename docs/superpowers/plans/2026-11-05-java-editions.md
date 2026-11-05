# Rusty Java Editions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement, validate and publish honest Fabric/NeoForge counterparts and a professional Modrinth page.

**Architecture:** Shared Java runtime with loader entry points and common game Mixins. Preserve native Rust Horizon product and immutable release. Fail-closed server capabilities govern restricted tools in Java distribution.

**Tech Stack:** Java25, Minecraft26.3, Gradle9.8.2, Fabric0.20.1/API0.165.2+26.3/Loom1.19.3, NeoForge26.3.0.101/ModDevGradle2.0.148.

**Spec:** `docs/superpowers/specs/2026-11-05-java-editions-design.md`

## Global Constraints

- Current Horizon source/package and player data remain intact.
- Exact Minecraft26.3 targets; no unsupported version tags.
- Client reads only received world state; bounded work, no packet exploits.
- Server capability state cannot be granted by a client profile.
- Publish only built and accepted Java artifacts; preserve legal notices.
- Native Horizon cannot be tagged as a Fabric/NeoForge jar on Modrinth.

## Review Focus

- New connection/dimension must not inherit stale target/entity/permission data.
- Closing a menu/releasing a hold key restores player control immediately.
- Unknown/unloaded blocks must not be treated as known ores or safe ground.
- User input/container state cancels automation; normal server reach/cooldowns apply.
- Saved settings/keybinding/HUD layout survive restart without changing other editions.

## Task 1: Shared runtime and loader contracts

Files: `java/{settings.gradle,build.gradle,gradle.properties}`, loader builds, `java/common/src/main/java/dev/justeck/rusty/core/*`, shared tests.

- [ ] Write behavioral tests for defaults/config bounds, connection-scoped grants, hold/release and bounded target lifecycle. Observe red.
- [ ] Implement configuration/catalog, capabilities and incremental scan/input models to pass.
- [ ] Configure official loaders, manifests and reproducible Gradle wrapper; compile both, fix actual API errors.

## Task 2: Game adapters and interface

Files: shared client, screens, HUD, scanning/rendering Mixins; Fabric and NeoForge entry points.

- [ ] Add fixtures for off/on restoration, chunk changes, settings/keybind persistence and ordinary actions before corresponding implementation.
- [ ] Port each module with bounded work and explicit ownership; enable only implemented entries. Keep a feature/evidence matrix current.
- [ ] Build both final jars and launch owned game fixtures; capture actual UI and inspect logs. Correct failures instead of treating compilation as runtime proof.

## Task 3: Server consent and multiplayer acceptance

Files: capability payload/receiver, optional server consent component, owned server fixtures.

- [ ] Test absent consent, grant/revoke, reconnect and malformed/oversized payloads.
- [ ] Implement minimal authoritative server opt-in and test server state changes against each Java loader.
- [ ] Record precise test environments and remaining limits; preserve raw receipts.

## Task 4: Review, publish and verify

Files: loader CI, root README/installation/support table, release notes, Modrinth description/gallery.

- [ ] Run final tests/compilation/artifact validation and request one independent whole-branch review; fix material findings.
- [ ] Update GitHub with edition-specific evidence and media, keep native installer compatibility.
- [ ] Publish accepted release assets; verify immutable downloads and checksums.
- [ ] Use signed-in Modrinth UI to create the complete project, correct environment/dependencies and supported versions, upload accepted jars and native-install link. Verify published/review status honestly.
