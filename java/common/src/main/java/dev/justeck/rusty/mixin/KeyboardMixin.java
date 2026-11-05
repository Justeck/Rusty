package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyClient;
import net.minecraft.client.KeyboardHandler;
import net.minecraft.client.input.KeyEvent;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(KeyboardHandler.class)
public abstract class KeyboardMixin {
  @Inject(method = "keyPress", at = @At("HEAD"))
  private void rusty$queuedPress(long window, int action, KeyEvent event, CallbackInfo ci) {
    if (action == 1) RustyClient.keyPressed(event.key());
  }
}
