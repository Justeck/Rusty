package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyAssistance;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Minecraft.class)
public abstract class UseKeyMixin {
  @Inject(method = "handleKeybinds", at = @At("HEAD"), cancellable = true)
  private void rusty$detachedInput(CallbackInfo ci) {
    if (dev.justeck.rusty.RustyClient.active("freecam")) {
      for (KeyMapping key : Minecraft.getInstance().options.keyMappings)
        while (key.consumeClick()) {}
      ci.cancel();
    }
  }

  @Redirect(
      method = "handleKeybinds",
      at = @At(value = "INVOKE", target = "Lnet/minecraft/client/KeyMapping;isDown()Z"))
  private boolean rusty$ordinaryFoodHold(KeyMapping key) {
    return key.isDown()
        || key == Minecraft.getInstance().options.keyUse && RustyAssistance.eating();
  }
}
