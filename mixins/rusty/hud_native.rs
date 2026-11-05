// Screen HUD vertex adapter; world-space gizmos never pass through this module.
use horizon_client::hud::HudBatch;
use horizon_client_render::mesher::rusty_hud_layout::HudLayout;
pub(super) use horizon_client_render::mesher::rusty_hud_render::{begin, finish, place};

pub(super) fn stacked_lines(
    game: &super::Game<'_>,
    batch: &mut HudBatch,
    layout: &HudLayout,
    groups: &[(&str, Vec<(String, u32)>)],
) {
    for (id, rows) in groups {
        lines(game, batch, layout, id, rows);
    }
}

pub(super) fn lines(
    game: &super::Game<'_>,
    batch: &mut HudBatch,
    layout: &HudLayout,
    id: &str,
    lines: &[(String, u32)],
) {
    let start = batch.vertices.len();
    draw_lines_at(game, batch, lines, [0.0, 0.0]);
    place(batch, start, layout, id);
}
pub(super) fn draw_lines_at(
    game: &super::Game<'_>,
    batch: &mut HudBatch,
    lines: &[(String, u32)],
    origin: [f32; 2],
) {
    if lines.is_empty() {
        return;
    }
    let width = lines
        .iter()
        .map(|(line, _)| game.native_text_width(line))
        .fold(0.0, f32::max)
        + 8.0;
    game.native_fill(
        batch,
        [origin[0], origin[1], width, lines.len() as f32 * 11.0 + 5.0],
        0xa0181715,
    );
    for (index, (line, color)) in lines.iter().enumerate() {
        game.native_text(
            batch,
            line,
            [origin[0] + 4.0, origin[1] + 3.0 + index as f32 * 11.0],
            *color,
            true,
        );
    }
}
