package dev.justeck.rusty.qa.mixin;

import dev.justeck.rusty.qa.GameAcceptance;
import net.minecraft.client.Minecraft;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(value = Minecraft.class, priority = 900)
public abstract class QaTickMixin {
  @Inject(method = "tick", at = @At("TAIL"))
  private void rusty$ownedFixture(CallbackInfo ci) {
    GameAcceptance.tick();
  }
}
