package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyClient;
import net.minecraft.client.renderer.Lightmap;
import net.minecraft.client.renderer.state.LightmapRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Lightmap.class)
public abstract class LightmapMixin {
  private boolean rusty$last;

  @Inject(method = "render", at = @At("HEAD"))
  private void rusty$bright(LightmapRenderState state, CallbackInfo ci) {
    boolean bright = RustyClient.active("fullbright") || RustyClient.view.xray();
    if (bright != rusty$last) state.needsUpdate = true;
    if (bright) {
      state.needsUpdate = true;
      state.brightness = 1;
      state.nightVisionEffectIntensity = 1;
      state.darknessEffectScale = 0;
      state.bossOverlayWorldDarkening = 0;
    }
    rusty$last = bright;
  }
}
