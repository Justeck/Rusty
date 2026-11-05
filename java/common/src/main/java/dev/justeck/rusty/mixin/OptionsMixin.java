package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyScreen;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** A recovery path remains available even when the menu shortcut is cleared. */
@Mixin(OptionsScreen.class)
public abstract class OptionsMixin extends Screen {
  protected OptionsMixin(Component title) {
    super(title);
  }

  @Inject(method = "init", at = @At("TAIL"))
  private void rusty$options(CallbackInfo ci) {
    addRenderableWidget(
        Button.builder(
                Component.literal("Rusty"), b -> minecraft.gui.setScreen(new RustyScreen(this)))
            .bounds(8, 8, 64, 20)
            .build());
  }
}
