package dev.justeck.rusty.mixin;

import dev.justeck.rusty.*;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.Hud;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Hud.class)
public abstract class HudMixin {
  @Inject(method = "extractRenderState", at = @At("TAIL"))
  private void rusty$hud(GuiGraphicsExtractor graphics, DeltaTracker delta, CallbackInfo ci) {
    if (!(net.minecraft.client.Minecraft.getInstance().gui.screen() instanceof HudEditor))
      RustyHud.draw(graphics, false);
  }
}
