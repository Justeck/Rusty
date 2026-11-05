package dev.justeck.rusty.fabric;

import dev.justeck.rusty.RustyClient;
import dev.justeck.rusty.network.ConsentPayload;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.model.loading.v1.ModelLoadingPlugin;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;

public final class RustyFabric implements ClientModInitializer {
  @Override
  public void onInitializeClient() {
    ModelLoadingPlugin.register(context ->
        context
            .modifyBlockModelAfterBake()
            .register((model, modelContext) -> new FabricFrameModel(model)));
    ClientPlayConnectionEvents.JOIN.register((handler, sender, client) -> {
      if (client.getConnection() != handler) return;
      ClientPlayNetworking.registerReceiver(
          ConsentPayload.TYPE,
          (payload, context) ->
              client.execute(() -> RustyClient.permissions(handler, payload.mask())));
    });
  }
}
