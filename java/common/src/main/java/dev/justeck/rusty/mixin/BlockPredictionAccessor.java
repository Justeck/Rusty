package dev.justeck.rusty.mixin;

import it.unimi.dsi.fastutil.longs.Long2ObjectOpenHashMap;
import net.minecraft.client.multiplayer.prediction.BlockStatePredictionHandler;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

/** Read-only observation of vanilla's normal server acknowledgments. */
@Mixin(BlockStatePredictionHandler.class)
public interface BlockPredictionAccessor {
  @Accessor("serverVerifiedStates")
  Long2ObjectOpenHashMap<?> rusty$pending();
}
