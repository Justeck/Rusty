package dev.justeck.rusty.qa.mixin;

import dev.justeck.rusty.qa.SchematicDamageEvidence;
import net.minecraft.client.multiplayer.ClientPacketListener;
import net.minecraft.network.protocol.game.*;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Independent read-only observer of real main-thread incoming damage/health handling. */
@Mixin(ClientPacketListener.class)
public abstract class SchematicDamageEvidenceMixin {
  @Inject(
      method = "handleDamageEvent",
      at =
          @At(
              value = "INVOKE",
              target =
                  "Lnet/minecraft/world/entity/Entity;handleDamageEvent(Lnet/minecraft/world/damagesource/DamageSource;)V"))
  private void rusty$damage(ClientboundDamageEventPacket packet, CallbackInfo ci) {
    SchematicDamageEvidence.receivedDamage((ClientPacketListener) (Object) this, packet.entityId());
  }

  @Inject(method = "handleSetHealth", at = @At("TAIL"))
  private void rusty$health(ClientboundSetHealthPacket packet, CallbackInfo ci) {
    SchematicDamageEvidence.receivedHealth((ClientPacketListener) (Object) this);
  }
}
