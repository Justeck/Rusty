// Added 2026-11-03 by Justeck, Apache-2.0.
// Reuse Horizon's invalidation: results dispatched under the old mode are rejected.
pub fn invalidate(streaming: &mut super::Streaming, world: &mut crate::world::World) {
    let context = streaming
        .shared
        .context
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    streaming.reload_resources(world, context);
}
