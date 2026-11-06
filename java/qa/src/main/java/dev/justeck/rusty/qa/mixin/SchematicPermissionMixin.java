package dev.justeck.rusty.qa.mixin;

import dev.justeck.rusty.qa.SchematicAcceptance;
import net.minecraft.client.multiplayer.ClientPacketListener;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Observes actual vanilla mode/ability receipts; never changes them. */
@Mixin(ClientPacketListener.class)
public abstract class SchematicPermissionMixin {
  @Inject(
      method = {"handleGameEvent", "handlePlayerAbilities"},
      at = @At("TAIL"))
  private void rusty$permission(CallbackInfo ci) {
    if ("1".equals(System.getenv("RUSTY_QA_SCHEMATICS"))) SchematicAcceptance.permissionReceipt();
  }
}
