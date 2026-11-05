package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyClient;
import net.minecraft.client.Minecraft;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Minecraft.class)
public abstract class MinecraftMixin {
  @Inject(method = "tick", at = @At("TAIL"))
  private void rusty$tick(CallbackInfo ci) {
    RustyClient.tick();
  }

  @Inject(method = "close", at = @At("HEAD"))
  private void rusty$save(CallbackInfo ci) {
    RustyClient.shutdown();
  }
}
