package dev.justeck.rusty.mixin;

import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.block.state.BlockState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Invoker;

/** Pure host placement-state computation, including its ordinary survival/collision rules. */
@Mixin(BlockItem.class)
public interface SchematicBlockItemAccessor {
  @Invoker("getPlacementState")
  BlockState rusty$placementState(BlockPlaceContext context);
}
