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
    if (action == 1) {
      dev.justeck.rusty.SchematicConstruction.manual();
      if (event.key()
          != RustyClient.config.bindings.getOrDefault(
              dev.justeck.rusty.RustyMining.areaActive() ? "area_clear" : "auto_mine", -1))
        dev.justeck.rusty.RustyMining.manual();
      RustyClient.keyPressed(event.key());
    }
  }
}
