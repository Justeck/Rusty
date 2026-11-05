# Rusty visual identity

Rusty uses an original squared R inside a beveled voxel badge. Warm copper, graphite recesses and small patina accents connect the name to Rust and a block-built world without copying the official Rust gear or Minecraft wordmark. The bold silhouette stays recognizable at small sizes; longer project text uses ordinary readable type.

| Asset | Use |
|---|---|
| [rusty-mark.png](rusty-mark.png) | Transparent standalone emblem |
| [rusty-wordmark.png](rusty-wordmark.png) | Transparent integrated [R badge]USTY wordmark |
| [rusty-cover.png](rusty-cover.png) | Original 2:1 repository cover |
| [rusty-cover.jpg](rusty-cover.jpg) | Encoding-only copy below GitHub's social-preview upload limit |

[branding.json](branding.json) records original dimensions, file hashes and processing. Transparent assets preserve the generated alpha channel. Cover artwork has an opaque graphite background. The JPEG preserves the original composition, with no cropping or content changes.

## Design briefs

The assets were produced using the built-in image-generation tool. These briefs record the shared direction:

- **Mark:** Original, legible, squared copper R inside a beveled octagonal voxel badge; dark graphite recesses, restrained green patina, clean transparent background. Avoid the official Rust gear, Minecraft wordmark and unrelated symbols.
- **Wordmark:** The badge itself is the R, followed only by USTY. Never place a second text R after the badge. Chunky squared letterforms with consistent copper bevels and modest patina detail; balanced spacing and a transparent background.
- **Cover:** A 2:1 dark graphite voxel-grid composition with the integrated [R badge]USTY wordmark. Readable subtitle: Horizon · Fabric · NeoForge. Small supporting text: 42 tools. One familiar interface. Keep the hierarchy simple and uncluttered.

Game screenshots document the actual renderer and interface. Branding is kept separate from those captures.

The native and Java menu headers use the standalone badge. Native distribution includes a 64px RGBA rasterization of the original mark for Horizon’s HUD atlas; it is uploaded once when opening the menu.
