package dev.justeck.rusty.mixin;

import dev.justeck.rusty.SchematicConstruction;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;

/** Actual received block application and prediction settlement, including unchanged states. */
@Mixin(ClientLevel.class)
public abstract class SchematicTerrainMixin {
  @Inject(method = "setServerVerifiedBlockState", at = @At("TAIL"))
  private void rusty$received(BlockPos pos, BlockState state, int flags, CallbackInfo ci) {
    SchematicConstruction.terrainColumn(this, pos.getX() >> 4, pos.getZ() >> 4);
  }

  @Inject(method = "syncBlockState", at = @At("TAIL"))
  private void rusty$settled(BlockPos pos, BlockState state, Vec3 playerPos, CallbackInfo ci) {
    SchematicConstruction.terrainColumn(this, pos.getX() >> 4, pos.getZ() >> 4);
  }

  @Inject(method = "setBlock", at = @At("RETURN"))
  private void rusty$predicted(
      BlockPos pos, BlockState state, int flags, int limit, CallbackInfoReturnable<Boolean> ci) {
    if (ci.getReturnValue())
      SchematicConstruction.terrainColumn(this, pos.getX() >> 4, pos.getZ() >> 4);
  }
}
