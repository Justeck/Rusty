package dev.justeck.rusty.neoforge;

import dev.justeck.rusty.network.*;
import net.neoforged.api.distmarker.Dist;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.fml.common.Mod;
import net.neoforged.fml.loading.FMLEnvironment;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.tick.ServerTickEvent;
import net.neoforged.neoforge.network.PacketDistributor;
import net.neoforged.neoforge.network.event.RegisterPayloadHandlersEvent;

@Mod("rusty")
public final class RustyNeoForge {
  public RustyNeoForge(IEventBus bus) {
    bus.addListener(this::payloads);
    NeoForge.EVENT_BUS.addListener(this::serverTick);
  }

  private void payloads(RegisterPayloadHandlersEvent event) {
    event
        .registrar("1")
        .optional()
        .playToClient(
            ConsentPayload.TYPE,
            ConsentPayload.CODEC,
            (payload, context) -> {
              if (FMLEnvironment.getDist() == Dist.CLIENT) ClientConsent.receive(payload, context);
            });
  }

  private void serverTick(ServerTickEvent.Post event) {
    ServerConsent.tick(
        event.getServer(),
        (player, payload) -> {
          if (!player.connection.hasChannel(ConsentPayload.TYPE.id())) return false;
          PacketDistributor.sendToPlayer(player, payload);
          return true;
        });
  }
}
