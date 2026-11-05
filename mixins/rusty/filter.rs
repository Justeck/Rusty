// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
// SDK359: filter the freshly refilled worker scratch, not the received chunks.
fn rusty_filter(value: (), pad: &mut Padded, region: &[[Option<&Chunk>; 3]; 3], sy: i32) {
    rusty::filter(
        &mut pad.blocks,
        &mut pad.light,
        region[1][1].map_or(MIN_Y, Chunk::min_y) + sy * 16,
        B,
        P,
    );
    value
}
