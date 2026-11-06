package dev.justeck.rusty.mixin;

import dev.justeck.rusty.SchematicConstruction;
import net.minecraft.client.multiplayer.ClientPacketListener;
import net.minecraft.network.protocol.game.*;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Only incoming vanilla packets enter receipt gates; local menu predictions never do. */
@Mixin(ClientPacketListener.class)
public abstract class SchematicReceiptMixin {
  @Inject(method = "handleDamageEvent", at = @At("TAIL"))
  private void rusty$damage(ClientboundDamageEventPacket p, CallbackInfo ci) {
    // TAIL is after vanilla's main-thread dispatch. Absorption or same-frame healing can leave
    // health unchanged, but this is still an authoritative damage receipt for this exact entity.
    var level = ((ClientPacketListener) (Object) this).getLevel();
    if (level != null)
      SchematicConstruction.damageReceipt(
          this, level.getEntity(p.entityId()), level, dev.justeck.rusty.RustyClient.scope());
  }

  @Inject(method = "handleContainerSetSlot", at = @At("TAIL"))
  private void rusty$slot(ClientboundContainerSetSlotPacket p, CallbackInfo ci) {
    if (p.getContainerId() == 0)
      SchematicConstruction.inventoryReceipt(this, p.getSlot(), p.getStateId(), p.getItem());
  }

  @Inject(method = "handleContainerContent", at = @At("TAIL"))
  private void rusty$inventory(ClientboundContainerSetContentPacket p, CallbackInfo ci) {
    if (p.containerId() == 0)
      for (int i = 0; i < p.items().size(); i++)
        SchematicConstruction.inventoryReceipt(this, i, p.stateId(), p.items().get(i));
  }

  @Inject(method = "handleBlockChangedAck", at = @At("TAIL"))
  private void rusty$ack(ClientboundBlockChangedAckPacket p, CallbackInfo ci) {
    SchematicConstruction.placementAck(this, p.sequence());
    SchematicConstruction.receivedRevision++;
  }

  @Inject(method = "handleAwardStats", at = @At("TAIL"))
  private void rusty$stats(ClientboundAwardStatsPacket p, CallbackInfo ci) {
    dev.justeck.rusty.SchematicStatsBarrier.received(
        ((ClientPacketListener) (Object) this).getConnection());
  }

  @Inject(method = "handleMovePlayer", at = @At("TAIL"))
  private void rusty$positionCorrection(ClientboundPlayerPositionPacket p, CallbackInfo ci) {
    dev.justeck.rusty.SchematicRotation.reset();
    SchematicConstruction.stop("Server position correction changed the construction pose");
  }

  @Inject(method = "handleRotatePlayer", at = @At("TAIL"))
  private void rusty$rotationCorrection(ClientboundPlayerRotationPacket p, CallbackInfo ci) {
    dev.justeck.rusty.SchematicRotation.reset();
    SchematicConstruction.stop("Server rotation correction changed the construction pose");
  }
}
