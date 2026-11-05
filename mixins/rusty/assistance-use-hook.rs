// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_assistance_use(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    crate::game::rusty_assist::use_item(game)
}
