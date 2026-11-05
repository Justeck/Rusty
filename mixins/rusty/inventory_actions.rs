// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
use super::Game;
use horizon_client::menu::Input;
pub(crate) fn click(game: &mut Game<'_>, source: usize, hotbar: Option<usize>) -> bool {
    if !game.was_playing
        || !game.native_playing()
        || game.remote.is_none()
        || game.remote_menu().0 != 0
        || game.carried.is_some()
        || source >= 36
    {
        return false;
    }
    if let Some(target) = hotbar {
        if target >= 9 || source < 9 || game.inventory[target].is_some() {
            return false;
        }
        game.remote_menu_ops(&[(Some(source), target as i32, Input::Swap)])
    } else {
        game.remote_menu_ops(&[(Some(source), 0, Input::QuickMove)])
    }
}
