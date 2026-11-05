# Rusty Java editions

## Intent and scope

Add real Fabric and NeoForge editions of Rusty's existing client tools for stable Minecraft 26.3. Keep the already accepted Horizon 1.2.0 archive, source pins, installation and game results intact. Reuse the visual identity and interaction design; never substitute Horizon screenshots or acceptance receipts for Java evidence.

## Architecture decision

Use one shared Java client implementation with small Fabric and NeoForge entry points and common Mixins, compiled independently against both loaders. The repository remains primarily Rust; Horizon's runtime stays Rust. A JNI rewrite was considered and rejected: serializing game snapshots and shipping per-OS native libraries would add overhead and failure modes without measured benefit. Independent duplicated Java ports were rejected because their behavior would drift.

## Behavior and distribution

Port the existing bounded tools, not the reference clients' larger unimplemented catalogs. Ordinary rendering, input, inventory and server authority must remain intact. Profiles, keybind clearing, toggle/hold activation, texture context, target editing, menu scaling, enabled collection and independently movable HUDs belong in the shared client. Scope caches to connection and dimension and bound scans/render work. Never request withheld data or bypass server restrictions.

Modrinth's published rules require explicit server opt-in for see-through rendering, automated combat and movement modifications. The Java distribution therefore fails closed on multiplayer connections unless an actual server-side capability message grants the corresponding feature. Singleplayer grants those capabilities locally. Permission state is transient, connection-bound, cleared on disconnect and never taken from client-editable profiles. Supply an optional server consent component and protocol documentation; ordinary information tools require no server component. Do not advertise this as an unrestricted hacked client on Modrinth or conceal the actual features. Modrinth currently has no Horizon loader, so the `.hnmod` remains a GitHub download, not a falsely tagged Java release.

## Validation and presentation

Pin stable tools from official metadata (JDK25, Fabric Loader0.20.1, Fabric API0.165.2+26.3, Loom1.19.3, NeoForge26.3.0.101, ModDevGradle2.0.148, Gradle9.8.2). Verify dependency compatibility by compiling, not version strings alone. Test shared configuration, permission lifecycle, filtering, cache bounds and inputs; launch both clients against disposable worlds and an owned test server where possible. Do not enter the player's personal worlds/public server. Publish exact supported loader/game versions and a per-edition feature/evidence table. Publish only accepted installable artifacts with checksums; no placeholders, broadened compatibility tags or claims of parity without evidence.

## Authorization and unresolved dependency

The user requested autonomous implementation and GitHub/Modrinth publication and has opted out of routine design questions. Choices above are implementation decisions, not additional claims of user approval. Previous GitHub repository deletion remains paused at a fresh email identity check; it does not block independent Java work. Authentication codes remain user-entered.
