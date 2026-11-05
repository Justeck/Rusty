package dev.justeck.rusty.fabric;

import dev.justeck.rusty.network.*;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.networking.v1.PayloadTypeRegistry;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;

public final class RustyFabricServer implements ModInitializer {
  @Override
  public void onInitialize() {
    PayloadTypeRegistry.clientboundPlay().register(ConsentPayload.TYPE, ConsentPayload.CODEC);
    ServerTickEvents.END_SERVER_TICK.register(server ->
        ServerConsent.tick(
            server,
            (player, payload) -> {
              if (!ServerPlayNetworking.canSend(player, ConsentPayload.TYPE)) return false;
              ServerPlayNetworking.send(player, payload);
              return true;
            }));
  }
}
