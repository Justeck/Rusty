package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyClient;
import net.minecraft.client.renderer.fog.FogData;
import net.minecraft.client.renderer.fog.FogRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(FogRenderer.class)
public abstract class FogMixin {
  @Inject(method = "setupFog", at = @At("RETURN"))
  private void rusty$clear(CallbackInfoReturnable<FogData> cir) {
    if (RustyClient.active("no_fog")) {
      var f = cir.getReturnValue();
      f.environmentalStart = f.renderDistanceStart = 100000;
      f.environmentalEnd = f.renderDistanceEnd = f.skyEnd = f.cloudEnd = 100001;
    }
  }
}
