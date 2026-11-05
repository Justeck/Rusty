// Rusty textured navigation frames, Copyright 2026 Justeck, Apache-2.0.
// Uses the player's existing baked block models and texture array; no textures
// are copied into this mod or obtained from the server beyond ordinary chunks.
use super::Game;
use horizon_client::mesher::{CULL_BACK, Vertex, rusty};
use horizon_core::{
    geometry::{Direction, ShapeOp},
    position::BlockPos,
};
use horizon_world::registry::BlockStateId;

#[derive(Clone)]
pub(super) struct Face {
    pub pos: [i32; 3],
    positions: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    rgb: u32,
}

pub(super) fn faces(
    game: &Game<'_>,
    pos: [i32; 3],
    state: u32,
    cfg: &rusty::Config,
    cat: &rusty::Catalog,
) -> Vec<Face> {
    let id = BlockStateId(state);
    let visibility = &game.pack.context.tables.visibility;
    let visible = Direction::ALL.map(|d| {
        let n = d.normal();
        let other: [i32; 3] = std::array::from_fn(|i| pos[i] + n[i]);
        if !game.world.loaded(other[0], other[2]) {
            return false;
        }
        let neighbor = game.world.block(other[0], other[1], other[2]);
        if super::rusty_visuals::selected(cat, cfg, neighbor.0, other[1]) {
            return true;
        }
        match (
            visibility.occlusion_face(id, d),
            visibility.occlusion_face(neighbor, d.opposite()),
        ) {
            (_, Some((1, _))) => false,
            (Some((0, _)), _) | (_, Some((0, _))) => true,
            (Some((_, own)), Some((_, other))) => own
                .join_is_not_empty(other, ShapeOp::ONLY_FIRST)
                .unwrap_or(true),
            _ => true,
        }
    });
    if game.res.facts().get(id).solid_render && !visible.iter().any(|v| *v) {
        return Vec::new();
    }
    let Ok(models) = game
        .pack
        .render
        .model_catalog
        .collect(id, BlockPos::new(pos[0], pos[1], pos[2]))
    else {
        return Vec::new();
    };
    let chunks = &game.world.chunks;
    let region = std::array::from_fn(|dz| {
        std::array::from_fn(|dx| {
            chunks
                .get(&((pos[0] >> 4) + dx as i32 - 1, (pos[2] >> 4) + dz as i32 - 1))
                .map(|c| &**c)
        })
    });
    let tints = game.pack.context.tables.block_tints(
        id,
        pos,
        &region,
        game.res.biomes(),
        game.streaming.biome_zoom,
        game.streaming.biome_blend,
    );
    let offset = game
        .pack
        .context
        .tables
        .offset(id, pos[0], pos[2])
        .unwrap_or([0.; 3]);
    let mut out = Vec::new();
    for (_, model) in models {
        for face in &model.faces {
            if face.translucent {
                continue;
            }
            let q = &face.quad;
            if q.cull.is_some_and(|d| !visible[d as usize]) {
                continue;
            }
            let Some(sprite) = game
                .pack
                .layer_names
                .get(&face.layer)
                .and_then(|name| game.gui.atlas.sprites.get(name))
            else {
                continue;
            };
            let rgb = tints(q.tint_index).unwrap_or(0xffffff);
            out.push(Face {
                pos,
                positions: q
                    .positions
                    .map(|p| std::array::from_fn(|i| p[i] + offset[i] as f32)),
                uv: q.uv.map(|uv| {
                    [
                        sprite.u0 + uv[0] * (sprite.u1 - sprite.u0),
                        sprite.v0 + uv[1] * (sprite.v1 - sprite.v0),
                    ]
                }),
                rgb,
            });
            if out.len() >= 32 {
                return out;
            }
        }
    }
    out
}

fn interpolate<const N: usize>(corners: [[f32; N]; 4], u: f32, v: f32) -> [f32; N] {
    std::array::from_fn(|i| {
        corners[0][i] * (1. - u) * (1. - v)
            + corners[1][i] * u * (1. - v)
            + corners[2][i] * u * v
            + corners[3][i] * (1. - u) * v
    })
}

pub(super) fn append_face(
    out: &mut Vec<Vertex>,
    face: &Face,
    camera: [f64; 3],
    cfg: &rusty::Config,
) {
    let rgb = face.rgb;
    let color = (u32::from(cfg.context_opacity) << 24)
        | ((rgb & 255) << 16)
        | (rgb & 0xff00)
        | ((rgb >> 16) & 255);
    for [u0, v0, u1, v1] in cfg.terrain_frame_rects() {
        for [u, v] in [[u0, v0], [u1, v0], [u1, v1], [u0, v1]] {
            let point = interpolate(face.positions, u, v);
            out.push(Vertex {
                pos: std::array::from_fn(|i| {
                    (f64::from(face.pos[i]) + f64::from(point[i]) - camera[i]) as f32
                }),
                uv: interpolate(face.uv, u, v),
                layer: 0,
                color,
                light: 0x0000_f0f0,
                normal: CULL_BACK,
                glint_uv: [0.; 2],
            });
        }
    }
}
