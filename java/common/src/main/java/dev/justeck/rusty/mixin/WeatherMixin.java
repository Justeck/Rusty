package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyClient;
import net.minecraft.client.multiplayer.ClientLevel;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(net.minecraft.world.level.Level.class)
public abstract class WeatherMixin {
  @Inject(
      method = {"getRainLevel", "getThunderLevel"},
      at = @At("RETURN"),
      cancellable = true)
  private void rusty$localWeather(float partial, CallbackInfoReturnable<Float> cir) {
    if ((Object) this instanceof ClientLevel && RustyClient.active("clear_weather"))
      cir.setReturnValue(0f);
  }
}
