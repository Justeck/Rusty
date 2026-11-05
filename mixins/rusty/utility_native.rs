// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// API359 read-only adapter. No inventory mutation, interactions or packets.
use super::Game;
use horizon_client::{hud::HudBatch, items::ItemStack, lang};
use horizon_client_render::mesher::{rusty, rusty_utility as utility};
use horizon_world::nbt::Tag;

const NORMAL: u32 = 0xffedddca;
const MUTED: u32 = 0xffb9b5af;
const WARNING: u32 = 0xffffb36b;

struct Line {
    text: String,
    color: u32,
}
fn tr(key: &str, fallback: &str) -> String {
    lang::get_or(key, fallback)
}
fn line(lines: &mut Vec<Line>, text: String, color: u32) {
    lines.push(Line { text, color });
}
fn item_name(item: &str) -> String {
    // Resource identifiers, not styled/custom names, keep the readout compact
    // and inert. Bound before friendly-name processing as well as afterward.
    let item = utility::bounded_label(item);
    utility::bounded_label(&rusty::friendly_name(&item))
}

fn durability(game: &Game<'_>, stack: &ItemStack) -> utility::Durability {
    let component = |id| stack.data().and_then(|data| data.other(id));
    let maximum = if component("!minecraft:max_damage").is_some() {
        utility::DamageMaximum::Removed
    } else {
        match component("minecraft:max_damage") {
            Some(Tag::Int(value)) => u32::try_from(*value)
                .ok()
                .map(utility::DamageMaximum::Value)
                .unwrap_or(utility::DamageMaximum::Unknown),
            // A malformed override must not silently become a vanilla maximum.
            Some(_) => utility::DamageMaximum::Unknown,
            None => game
                .items()
                .get(&stack.item)
                .map(|item| utility::DamageMaximum::Value(item.max_damage))
                .unwrap_or(utility::DamageMaximum::Unknown),
        }
    };
    utility::Durability::observe(
        stack.damage,
        maximum,
        component("!minecraft:damage").is_some(),
        component("minecraft:unbreakable").is_some(),
    )
}

fn equipment(game: &Game<'_>, settings: &utility::UtilitySettings, lines: &mut Vec<Line>) {
    let slots = [
        ("rusty.utility.head", "Head", game.armor[0].as_ref()),
        ("rusty.utility.chest", "Chest", game.armor[1].as_ref()),
        ("rusty.utility.legs", "Legs", game.armor[2].as_ref()),
        ("rusty.utility.feet", "Feet", game.armor[3].as_ref()),
        (
            "rusty.utility.main_hand",
            "Hand",
            game.inventory.get(game.selected).and_then(Option::as_ref),
        ),
        ("rusty.utility.off_hand", "Offhand", game.offhand.as_ref()),
    ];
    for (key, fallback, stack) in slots {
        let label = tr(key, fallback);
        let Some(stack) = stack.filter(|s| s.count > 0 && &*s.item != "minecraft:air") else {
            line(lines, format!("{label}: --"), MUTED);
            continue;
        };
        let durability = durability(game, stack);
        let detail = match durability {
            utility::Durability::Remaining { remaining, maximum } => {
                format!(
                    "{remaining}/{maximum} ({}%)",
                    durability.percentage().unwrap_or(0)
                )
            }
            utility::Durability::Unbreakable => tr("rusty.utility.unbreakable", "unbreakable"),
            utility::Durability::Unknown => {
                tr("rusty.utility.durability_unknown", "durability unknown")
            }
            utility::Durability::NotDamageable => format!("x{}", stack.count),
        };
        // Put numeric durability before the name, so narrow HUDs keep it.
        let color = if durability.warning(settings.warning_percent) {
            WARNING
        } else {
            NORMAL
        };
        line(
            lines,
            format!("{label}: {detail}  {}", item_name(&stack.item)),
            color,
        );
    }
}

fn environment(game: &Game<'_>, at: [f64; 3], lines: &mut Vec<Line>) {
    let biome_label = tr("rusty.utility.biome", "Biome");
    let light_label = tr("rusty.utility.local_light", "Light at feet");
    let unavailable = tr("rusty.utility.unavailable", "unavailable");
    let Some(pos) = utility::block_position(at) else {
        line(lines, format!("{biome_label}: {unavailable}"), MUTED);
        line(lines, format!("{light_label}: {unavailable}"), MUTED);
        return;
    };
    let chunk = game.world.chunks.get(&(pos[0] >> 4, pos[2] >> 4));
    let inside = pos[1] >= game.world.bottom_y() && pos[1] < game.world.top_y();
    let biome = chunk
        .filter(|c| inside && !c.sections.is_empty())
        .and_then(|chunk| {
            let index = chunk.biome(pos[0] & 15, pos[1], pos[2] & 15);
            // Avoid Biomes::get's fallback-to-last behavior for unknown IDs.
            let biomes = game
                .world_biomes
                .as_ref()
                .unwrap_or_else(|| game.res.biomes());
            biomes
                .list
                .get(usize::from(index))
                .map(|b| utility::bounded_label(&b.name))
        });
    line(
        lines,
        format!(
            "{biome_label}: {}",
            biome.as_deref().unwrap_or(&unavailable)
        ),
        if biome.is_some() { NORMAL } else { MUTED },
    );
    let ready = chunk.is_some_and(|c| c.lit || c.light_loaded)
        && !game.world.dirty_light.contains(&(pos[0] >> 4, pos[2] >> 4));
    // The native helper returns (0,0) for absent chunks; do not expose that
    // sentinel as measured darkness. Out-of-height light is also unknown here.
    let packed = chunk.map_or(0, |c| c.light(pos[0] & 15, pos[1], pos[2] & 15));
    let sample = utility::observed_light(packed, chunk.is_some(), inside, ready);
    if let Some(sample) = sample {
        let block = tr("rusty.utility.block_light", "block");
        let sky = tr("rusty.utility.sky_light", "sky");
        line(
            lines,
            format!(
                "{light_label}: {block} {} / {sky} {}",
                sample.block, sample.sky
            ),
            NORMAL,
        );
    } else {
        line(lines, format!("{light_label}: {unavailable}"), MUTED);
    }
}

fn container(game: &Game<'_>, settings: &utility::UtilitySettings, lines: &mut Vec<Line>) {
    let title = tr("rusty.utility.held_contents", "Held contents");
    // Read the selected main-hand stack even if the last acting hand was the
    // offhand. No chest lookup, cache, item use, hover interaction or recursion.
    let stack = game.inventory.get(game.selected).and_then(Option::as_ref);
    let data = stack.and_then(ItemStack::data);
    let Some(data) = data.filter(|d| !d.container.is_empty()) else {
        // SDK StackData flattens both absent and empty container components
        // to an empty Vec, so neither is advertised as a known-empty container.
        line(
            lines,
            format!(
                "{title}: {}",
                tr("rusty.utility.contents_unknown", "empty or not provided")
            ),
            MUTED,
        );
        return;
    };
    let input: Vec<_> = data
        .container
        .iter()
        .take(utility::MAX_PREVIEW_SCAN)
        .map(|(slot, item)| utility::PreviewInput {
            slot: *slot,
            item: &item.item,
            count: item.count,
        })
        .collect();
    let selected = utility::select_preview(
        &input,
        usize::from(settings.preview_rows),
        data.container.len() > utility::MAX_PREVIEW_SCAN,
    );
    if selected.indices.is_empty() {
        line(
            lines,
            format!(
                "{title}: {}",
                tr("rusty.utility.contents_unknown", "empty or not provided")
            ),
            MUTED,
        );
        return;
    }
    line(
        lines,
        format!(
            "{title} ({})",
            tr("rusty.utility.provided_items", "provided items")
        ),
        NORMAL,
    );
    for index in selected.indices {
        let entry = &input[index];
        line(
            lines,
            format!(
                "{}: x{} {}",
                usize::from(entry.slot) + 1,
                entry.count,
                item_name(entry.item)
            ),
            NORMAL,
        );
    }
    if selected.truncated {
        line(
            lines,
            tr(
                "rusty.utility.preview_partial",
                "More or invalid entries omitted",
            ),
            MUTED,
        );
    }
}

/// No cache is retained: all reads are constant/bounded in the current client
/// world, so there is no prior-server equipment, light or container to clear.
#[allow(dead_code)]
pub(super) fn frame(_game: &mut Game<'_>) {}
#[allow(dead_code)]
pub(super) fn reset() {}

#[allow(dead_code)]
pub(super) fn hud(game: &Game<'_>, batch: &mut HudBatch) {
    if !game.native_window()
        || !game.native_on(rusty::ID)
        || game.ui.hide_gui
        || game.title.is_some()
        || !game.native_playing()
    {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let cfg = rusty::config();
    let settings = &cfg.utilities;
    if settings.validate().is_err()
        || (!settings.equipment_hud && !settings.environment_hud && !settings.container_preview)
    {
        return;
    }
    let mut lines = Vec::with_capacity(20);
    if settings.equipment_hud {
        equipment(game, settings, &mut lines);
    }
    let equipment_end = lines.len();
    if settings.environment_hud {
        environment(game, [player.x, player.y, player.z], &mut lines);
    }
    let environment_end = lines.len();
    if settings.container_preview {
        container(game, settings, &mut lines);
    }
    for (panel, start, end) in [
        ("equipment_hud", 0, equipment_end),
        ("environment_hud", equipment_end, environment_end),
        ("container_preview", environment_end, lines.len()),
    ] {
        let rows: Vec<_> = lines[start..end]
            .iter()
            .map(|row| (row.text.clone(), row.color))
            .collect();
        super::rusty_hud_native::lines(game, batch, &cfg.client.hud_layout, panel, &rows);
    }
}
