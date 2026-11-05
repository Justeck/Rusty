package dev.justeck.rusty.mixin;

import dev.justeck.rusty.*;
import net.minecraft.client.Camera;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;

@Mixin(Camera.class)
public abstract class CameraMixin {
  @Shadow
  protected abstract void setPosition(Vec3 position);

  @Shadow
  protected abstract void setRotation(float yaw, float pitch);

  @Inject(
      method = "update",
      at = @At(value = "INVOKE", target = "Lnet/minecraft/client/Camera;calculateFov(F)F"))
  private void rusty$detached(DeltaTracker delta, CallbackInfo ci) {
    if (RustyClient.active("freecam")) {
      setPosition(RustyCamera.position);
      setRotation(RustyCamera.yaw, RustyCamera.pitch);
    }
  }

  @Inject(method = "calculateFov", at = @At("RETURN"), cancellable = true)
  private void rusty$zoom(float partial, CallbackInfoReturnable<Float> cir) {
    if (RustyClient.zoomHeld)
      cir.setReturnValue(cir.getReturnValueF() / (float) RustyClient.config.zoomFactor);
  }

  @Inject(method = "extractRenderState", at = @At("TAIL"))
  private void rusty$culling(CameraRenderState state, DeltaTracker delta, CallbackInfo ci) {
    if (RustyClient.view.xray()) state.smartCull = false;
    if (RustyClient.active("freecam")) state.isFirstPerson = false;
  }

  @Inject(method = "isDetached", at = @At("RETURN"), cancellable = true)
  private void rusty$detachedFlag(CallbackInfoReturnable<Boolean> cir) {
    if (RustyClient.active("freecam")) cir.setReturnValue(true);
  }
}
