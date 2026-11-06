package dev.justeck.rusty.mixin;

import dev.justeck.rusty.SchematicStatsBarrier;
import io.netty.channel.ChannelFutureListener;
import net.minecraft.network.Connection;
import net.minecraft.network.protocol.Packet;
import net.minecraft.network.protocol.game.ServerboundClientCommandPacket;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Covers every normal Stats UI/other caller request, not just construction's requests. */
@Mixin(Connection.class)
public abstract class SchematicStatsRequestMixin {
  @Inject(
      method =
          "send(Lnet/minecraft/network/protocol/Packet;Lio/netty/channel/ChannelFutureListener;Z)V",
      at = @At("HEAD"))
  private void rusty$ordinaryObservations(
      Packet<?> packet, ChannelFutureListener listener, boolean flush, CallbackInfo ci) {
    if (packet instanceof net.minecraft.network.protocol.game.ServerboundMovePlayerPacket movement)
      dev.justeck.rusty.SchematicRotation.sent((Connection) (Object) this, movement);
    if (packet instanceof net.minecraft.network.protocol.game.ServerboundUseItemPacket air)
      dev.justeck.rusty.SchematicRotation.airUse(
          (Connection) (Object) this, air.yRot(), air.xRot());
    if (packet instanceof ServerboundClientCommandPacket command
        && command.getAction() == ServerboundClientCommandPacket.Action.REQUEST_STATS)
      SchematicStatsBarrier.sent((Connection) (Object) this);
  }
}
