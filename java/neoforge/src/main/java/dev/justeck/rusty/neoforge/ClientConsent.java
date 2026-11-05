package dev.justeck.rusty.neoforge;

import dev.justeck.rusty.RustyClient;
import dev.justeck.rusty.network.ConsentPayload;
import net.neoforged.neoforge.network.handling.IPayloadContext;

final class ClientConsent {
  static void receive(ConsentPayload payload, IPayloadContext context) {
    var sender = context.listener();
    context.enqueueWork(() -> RustyClient.permissions(sender, payload.mask()));
  }
}
