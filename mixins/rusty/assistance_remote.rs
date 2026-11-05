// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Access ordinary native client actions in their owning module.
use super::Game;

pub(crate) fn attack(game: &mut Game<'_>) {
    game.remote_send_carried();
    game.remote_start_attack();
}

pub(crate) fn attack_strength(game: &mut Game<'_>) -> f32 {
    let speed = game
        .own_attribute("minecraft:attack_speed")
        .unwrap_or_else(|| game.main_hand_attack_speed());
    game.hand_strength.scale(0.5, speed)
}

pub(crate) fn use_food(game: &mut Game<'_>) {
    use horizon_client::first_person::Hand;
    game.use_delay = 4;
    if game.hands_busy() || game.native_interaction(1, Hand::Main).is_some() {
        return;
    }
    game.remote_send_carried();
    if game.remote_use_item(Hand::Main) {
        game.heights.item_used(Hand::Main);
    }
}

pub(crate) fn release_food(game: &mut Game<'_>) {
    game.release_remote_use();
}
