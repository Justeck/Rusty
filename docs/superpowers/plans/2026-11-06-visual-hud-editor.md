# Visual HUD editing

User requested a game-view button replacing coordinate controls, direct dragging,
image-like resizing, and Enter to confirm and return to Rusty's menu. Earlier
instructions authorize autonomous implementation, installation and publication.

## Design

Keep normalized saved positions for resolution independence, but hide coordinates
from the interface. Reuse each loader's actual HUD geometry for picking. Corners
resize proportionally, preserving readable text. Enter/Done commits a draft;
Escape/Cancel discards it. Preserve feature states. Block gameplay inputs while
editing. Keep the normal world visible. Default panel and Default layout remain.
Native Rust is primary; provide the equivalent interaction on Java editions.

## Tasks

1. Add geometry acceptance tests for actual-bounds picking, drag offset, four
   corner anchors, size limits, finite values, viewport changes and cancellation.
   Implement the Rust editor core and Java geometry counterpart; run full suites.
2. Record native HUD bounds, add a temporary layout preview, route screen inputs,
   replace coordinate sliders with Edit HUD, and save only on confirmation.
3. Upgrade Java HudEditor with handles, draft/cancel, Enter and actual-view previews.
   Add native/Java real-game acceptance for drag, resize, save/cancel and inputs.
4. Build supported native and Java packages; inspect actual game screenshots,
   preserve user settings, update documentation/releases, publish and install.

## Review focus

Viewport/menu scale independence; resize anchors; overlapping panels; hidden
overlays not enabled by editing; draft isolation; no actions reaching gameplay;
return to same menu; no modified player worlds or running personal sessions.
