package dev.justeck.rusty.mixin;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.justeck.rusty.*;
import net.minecraft.client.renderer.GameRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(GameRenderer.class)
public abstract class GameRendererMixin {
  @Inject(method = "renderItemInHand", at = @At("HEAD"), cancellable = true)
  private void rusty$detachedHands(CallbackInfo ci) {
    if (RustyClient.active("freecam")) ci.cancel();
  }

  @Inject(method = "bobHurt", at = @At("HEAD"), cancellable = true)
  private void rusty$hurt(CameraRenderState state, PoseStack pose, CallbackInfo ci) {
    if (RustyClient.active("no_hurt_camera")) ci.cancel();
  }

  @Inject(method = "renderLevel", at = @At("HEAD"))
  private void rusty$overlays(CallbackInfo ci) {
    RustyOverlay.render();
  }
}
