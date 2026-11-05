package dev.justeck.rusty.mixin;

import dev.justeck.rusty.RustyClient;
import net.minecraft.client.renderer.chunk.VisGraph;
import net.minecraft.core.BlockPos;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(VisGraph.class)
public abstract class VisibilityMixin {
  @Inject(method = "setOpaque", at = @At("HEAD"), cancellable = true)
  private void rusty$visibleSections(BlockPos pos, CallbackInfo ci) {
    if (RustyClient.view.xray()) ci.cancel();
  }
}
