package dev.justeck.rusty.qa.mixin;

import dev.justeck.rusty.qa.SchematicAcceptance;
import net.minecraft.network.protocol.PacketUtils;
import net.minecraft.network.protocol.game.ServerboundContainerClickPacket;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import net.minecraft.world.inventory.ContainerInput;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * Owned integrated fixture rejection. No production hook, synthetic client receipt or success
 * update.
 */
@Mixin(ServerGamePacketListenerImpl.class)
public abstract class SchematicRejectSwapMixin {
  @Inject(method = "handleContainerClick", at = @At("HEAD"), cancellable = true)
  private void rusty$ownedRejection(ServerboundContainerClickPacket packet, CallbackInfo ci) {
    if (!"1".equals(System.getenv("RUSTY_QA_SCHEMATICS"))
        || !SchematicAcceptance.rejectNextSwap
        || packet.containerInput() != ContainerInput.SWAP
        || packet.containerId() != 0) return;
    var listener = (ServerGamePacketListenerImpl) (Object) this;
    PacketUtils.ensureRunningOnSameThread(packet, listener, listener.player.level());
    if (listener.player.level().getServer()
        != net.minecraft.client.Minecraft.getInstance().getSingleplayerServer())
      throw new IllegalStateException("Fixture refuses non-owned server");
    SchematicAcceptance.rejectNextSwap = false;
    SchematicAcceptance.rejectedInventory++;
    listener.player.inventoryMenu.broadcastFullState();
    ci.cancel();
  }

  @Redirect(
      method = "handleUseItemOn",
      at =
          @At(
              value = "INVOKE",
              target =
                  "Lnet/minecraft/server/level/ServerPlayerGameMode;useItemOn(Lnet/minecraft/server/level/ServerPlayer;Lnet/minecraft/world/level/Level;Lnet/minecraft/world/item/ItemStack;Lnet/minecraft/world/InteractionHand;Lnet/minecraft/world/phys/BlockHitResult;)Lnet/minecraft/world/InteractionResult;"))
  private net.minecraft.world.InteractionResult rusty$rejectUse(
      net.minecraft.server.level.ServerPlayerGameMode mode,
      net.minecraft.server.level.ServerPlayer player,
      net.minecraft.world.level.Level level,
      net.minecraft.world.item.ItemStack stack,
      net.minecraft.world.InteractionHand hand,
      net.minecraft.world.phys.BlockHitResult hit) {
    if ("1".equals(System.getenv("RUSTY_QA_SCHEMATICS"))
        && SchematicAcceptance.rejectNextPlacement) {
      if (level.getServer() != net.minecraft.client.Minecraft.getInstance().getSingleplayerServer())
        throw new IllegalStateException("Fixture refuses non-owned server");
      SchematicAcceptance.rejectNextPlacement = false;
      SchematicAcceptance.rejectedPlacements++;
      // Vanilla's unmodified packet handler sends its normal block correction and sequence ACK.
      return net.minecraft.world.InteractionResult.FAIL;
    }
    return mode.useItemOn(player, level, stack, hand, hit);
  }
}
