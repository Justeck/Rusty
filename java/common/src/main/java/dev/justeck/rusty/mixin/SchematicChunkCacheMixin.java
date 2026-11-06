package dev.justeck.rusty.mixin;

import dev.justeck.rusty.SchematicConstruction;
import net.minecraft.client.multiplayer.*;
import net.minecraft.network.protocol.game.ClientboundLevelChunkPacketData;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.chunk.LevelChunk;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;

/** Cache operations cover direct column replacement, unload, and view-range eviction. */
@Mixin(ClientChunkCache.class)
public abstract class SchematicChunkCacheMixin {
  @Shadow @Final private ClientLevel level;

  @Inject(method = "replaceWithPacketData", at = @At("RETURN"))
  private void rusty$column(
      int x, int z, ClientboundLevelChunkPacketData data, CallbackInfoReturnable<LevelChunk> ci) {
    if (ci.getReturnValue() != null) SchematicConstruction.terrainColumn(level, x, z);
  }

  @Inject(method = "drop", at = @At("TAIL"))
  private void rusty$drop(ChunkPos pos, CallbackInfo ci) {
    SchematicConstruction.terrainColumn(level, pos.x(), pos.z());
  }

  @Inject(
      method = {"updateViewCenter", "updateViewRadius"},
      at = @At("TAIL"))
  private void rusty$range(CallbackInfo ci) {
    SchematicConstruction.terrainAll(level);
  }
}
