// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_schematic_stats_send(
    game: &crate::game::Game<'_>,
    packet: &horizon_protocol::packets::play::Serverbound,
) {
    if game.remote.is_some()
        && matches!(packet,horizon_protocol::packets::play::Serverbound::ClientCommand(p) if p.action==1)
    {
        crate::game::rusty_schematic_native::stats_sent();
    }
    if game.remote.is_some()
        && matches!(
            packet,
            horizon_protocol::packets::play::Serverbound::UseItemOn(_)
        )
    {
        crate::game::rusty_schematic_native::block_use_sent();
    }
}
