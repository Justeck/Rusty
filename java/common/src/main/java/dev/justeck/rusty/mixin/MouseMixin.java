package dev.justeck.rusty.mixin;

import dev.justeck.rusty.*;
import net.minecraft.client.MouseHandler;
import net.minecraft.client.player.LocalPlayer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(MouseHandler.class)
public abstract class MouseMixin {
  @Inject(method = "onButton", at = @At("HEAD"))
  private void rusty$observedClick(
      long window, net.minecraft.client.input.MouseButtonInfo button, int action, CallbackInfo ci) {
    if (action == 1 && net.minecraft.client.Minecraft.getInstance().gui.screen() == null)
      RustyHud.click();
  }

  @Redirect(
      method = "turnPlayer",
      at = @At(value = "INVOKE", target = "Lnet/minecraft/client/player/LocalPlayer;turn(DD)V"))
  private void rusty$cameraOnly(LocalPlayer player, double x, double y) {
    if (RustyClient.active("freecam")) RustyCamera.turn(x, y);
    else player.turn(x, y);
  }
}
