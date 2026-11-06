// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
fn rusty_schematic_stats_receive(
    game: &mut crate::game::Game<'_>,
    packet: &horizon_protocol::packets::play::Clientbound,
) {
    if matches!(
        packet,
        horizon_protocol::packets::play::Clientbound::AwardStats(_)
    ) {
        crate::game::rusty_schematic_native::stats_received();
    }
    if let horizon_protocol::packets::play::Clientbound::DamageEvent(p) = packet {
        crate::game::rusty_schematic_native::damage_received(game, p.id);
    }
}
