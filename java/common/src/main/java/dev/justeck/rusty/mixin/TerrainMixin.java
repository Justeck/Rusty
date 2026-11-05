package dev.justeck.rusty.mixin;

import dev.justeck.rusty.*;
import net.minecraft.client.renderer.block.BlockAndTintGetter;
import net.minecraft.client.renderer.block.BlockQuadOutput;
import net.minecraft.client.renderer.block.ModelBlockRenderer;
import net.minecraft.client.renderer.block.dispatch.BlockStateModel;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.state.BlockState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(ModelBlockRenderer.class)
public abstract class TerrainMixin {
  @Unique private boolean rusty$target;

  @ModifyVariable(method = "tesselateBlock", at = @At("HEAD"), argsOnly = true, ordinal = 0)
  private BlockQuadOutput rusty$textures(
      BlockQuadOutput output,
      BlockQuadOutput ignored,
      float x,
      float y,
      float z,
      BlockAndTintGetter level,
      BlockPos pos,
      BlockState state,
      BlockStateModel model,
      long seed) {
    var view = RustyClient.view;
    rusty$target = view.xray() && view.selected(state, pos);
    if (!view.xray() || rusty$target) return output;
    if (!view.context()) return (a, b, c, q, i) -> {};
    return (a, b, c, q, i) ->
        TextureFrames.emit(q, view.edge(), quad -> output.put(a, b, c, quad, i));
  }

  @Inject(method = "shouldRenderFace", at = @At("HEAD"), cancellable = true)
  private void rusty$targetFaces(CallbackInfoReturnable<Boolean> cir) {
    if (RustyClient.view.xray() && rusty$target) cir.setReturnValue(true);
  }
}
