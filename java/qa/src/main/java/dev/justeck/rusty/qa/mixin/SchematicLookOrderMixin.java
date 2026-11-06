package dev.justeck.rusty.qa.mixin;

import dev.justeck.rusty.qa.SchematicLookEvidence;
import io.netty.channel.ChannelFutureListener;
import net.minecraft.network.Connection;
import net.minecraft.network.protocol.Packet;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Observes actual packet order only. It creates/sends no packet and cannot authorize actions. */
@Mixin(Connection.class)
public abstract class SchematicLookOrderMixin {
  @Inject(
      method =
          "send(Lnet/minecraft/network/protocol/Packet;Lio/netty/channel/ChannelFutureListener;Z)V",
      at = @At("HEAD"))
  private void rusty$actualLookOrder(
      Packet<?> packet, ChannelFutureListener listener, boolean flush, CallbackInfo ci) {
    SchematicLookEvidence.observe((Connection) (Object) this, packet);
  }
}
