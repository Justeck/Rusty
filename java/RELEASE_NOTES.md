# Rusty Java 1.3.0-beta.5

**Entity Tracers now has independent target choices:** Players, Mobs and Dropped Items. Open **Right Shift → Entity Tracers**, select any combination and enable the feature. The selections are saved in profiles, and each has a Default button.

Entity ESP keeps its own target choices. For example, show boxes around mobs while tracing dropped items. Changing or disabling one feature leaves the other's choices intact. Finder's block tracers and X-ray terrain rendering remain separate.

Older Java profiles inherit their previous shared ESP target choices once. New profiles select players only; Entity Tracers remains disabled by default. Empty selections draw nothing, and unknown entity kinds stay excluded. The received-entity range and server permission policy remain in force. A dedicated 128-line entity-tracer budget prevents ESP boxes or Finder geometry from crowding out tracers; the existing main geometry budget stays 768.

Fabric and NeoForge for **Minecraft 26.3 / Java 25**. Install one matching loader jar. Fabric needs Loader 0.20.1+ and Fabric API 0.165.2+26.3; NeoForge targets 26.3.0.101. Restricted multiplayer tools continue to require explicit server opt-in. No fresh remote-server acceptance is claimed for this update.

[Validation](https://github.com/Justeck/Rusty/blob/main/docs/JAVA_VALIDATION.md) · [Separate Horizon-native edition](https://github.com/Justeck/Rusty/releases/tag/v1.2.6).
