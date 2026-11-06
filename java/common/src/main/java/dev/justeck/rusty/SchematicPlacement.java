package dev.justeck.rusty;

import dev.justeck.rusty.core.SchematicSafety;
import dev.justeck.rusty.mixin.SchematicBlockItemAccessor;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.*;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.*;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.*;
import net.minecraft.world.phys.*;

/**
 * Ordinary reach/aim/support probing. Only known vanilla block-item write patterns are dispatched.
 */
public final class SchematicPlacement {
  public record Probe(
      BlockHitResult hit, Map<BlockPos, BlockState> writes, float yaw, float pitch) {}

  public static int work;
  private static final Map<Class<?>, Boolean> PASSIVE_SUPPORT = new HashMap<>();

  private static boolean interactionFree(BlockState state) {
    return PASSIVE_SUPPORT.computeIfAbsent(
        state.getBlock().getClass(),
        type -> {
          for (Class<?> current = type;
              current != null
                  && current != net.minecraft.world.level.block.state.BlockBehaviour.class;
              current = current.getSuperclass())
            for (var method : current.getDeclaredMethods())
              if (method.getName().equals("useItemOn") || method.getName().equals("useWithoutItem"))
                return false;
          return true;
        });
  }

  public static boolean compatible(BlockState target, BlockState predicted) {
    return SchematicSafety.progress(
        RustySchematics.describe(target), RustySchematics.describe(predicted));
  }

  public static boolean replaceable(BlockPos at, BlockState wanted) {
    var mc = Minecraft.getInstance();
    var current = mc.level.getBlockState(at);
    return current.isAir()
        || current.getBlock() == wanted.getBlock()
            && !current.equals(wanted)
            && compatible(wanted, current);
  }

  private static boolean protectedNeighbours(BlockPos at, BlockState wanted, Set<BlockPos> writes) {
    var mc = Minecraft.getInstance();
    for (Direction direction : Direction.values()) {
      var neighbour = at.relative(direction);
      if (!SchematicConstruction.received(neighbour)) return false;
      if (writes.contains(neighbour)) continue;
      var current = mc.level.getBlockState(neighbour);
      if (current.getBlock() instanceof DoorBlock
          && (wanted.isSignalSource()
              || mc.level.hasNeighborSignal(at)
              || mc.level.hasNeighborSignal(neighbour))) return false;
      if (!stableNeighbour(
          current,
          wanted,
          direction.getOpposite(),
          RustySchematics.targets().containsKey(neighbour))) return false;
    }
    return true;
  }

  static BlockHitResult supportHit(
      Vec3 eye,
      Vec3 point,
      Direction face,
      BlockPos support,
      double reach,
      java.util.function.Function<Vec3, BlockHitResult> trace) {
    // The intended face must lie strictly inside the segment: host AABB.clip excludes t==1.
    var through =
        point.add(-face.getStepX() * .001, -face.getStepY() * .001, -face.getStepZ() * .001);
    var ray = trace.apply(through);
    return ray != null
            && ray.getType() == HitResult.Type.BLOCK
            && ray.getBlockPos().equals(support)
            && ray.getDirection() == face
            && ray.getLocation().distanceTo(eye) <= reach
        ? ray
        : null;
  }

  static boolean stableNeighbour(
      BlockState current, BlockState replacement, Direction fromNeighbour, boolean planned) {
    if (current.isAir()) return true;
    var block = current.getBlock();
    if (!net.minecraft.core.registries.BuiltInRegistries.BLOCK
            .getKey(block)
            .toString()
            .startsWith("minecraft:")
        || !block.getClass().getPackageName().equals("net.minecraft.world.level.block"))
      return false;
    // Known shape updates of these received neighbours do not react along this relation.
    if (block instanceof DoorBlock)
      return fromNeighbour.getAxis() != Direction.Axis.Y && !replacement.isSignalSource();
    if (block instanceof AbstractBedBlock)
      return fromNeighbour != AbstractBedBlock.getConnectedDirection(current);
    if (block instanceof StairBlock)
      return !(replacement.getBlock() instanceof StairBlock)
          && !current.getValue(BlockStateProperties.WATERLOGGED);
    // Vanilla dry slabs retain their shape/state on every neighbour update. Wet slabs schedule
    // fluid work and remain protected; custom subclasses were excluded above.
    if (block.getClass() == SlabBlock.class) return !current.getValue(SlabBlock.WATERLOGGED);
    // Membership in the plan never authorizes an unenumerated reactive/container write.
    if (!(block.getClass() == Block.class
            || block instanceof RotatedPillarBlock
            || block instanceof TransparentBlock
            || block instanceof StainedGlassBlock
            || block instanceof TintedGlassBlock
            || block instanceof GrassBlock
            || block instanceof DropExperienceBlock)
        || current.hasBlockEntity()
        || !current.getFluidState().isEmpty()) return false;
    for (var property : current.getProperties())
      if (!property.getName().equals("axis") && !property.getName().equals("snowy")) return false;
    return !current.hasProperty(BlockStateProperties.SNOWY)
        || !(replacement.is(Blocks.SNOW) || replacement.is(Blocks.SNOW_BLOCK));
  }

  static List<Vec3> supportPoints(BlockPos support, AABB box, Direction face) {
    var points = new ArrayList<Vec3>();
    if (face.getAxis() == Direction.Axis.Y) {
      double y = support.getY() + (face == Direction.UP ? box.maxY : box.minY);
      points.add(
          new Vec3(
              support.getX() + (box.minX + box.maxX) / 2,
              y,
              support.getZ() + (box.minZ + box.maxZ) / 2));
      // The old UP/DOWN height loop repeated the same center three times. Spend those four
      // redundant attempts on distinct top-face quadrants for ordinary direction/hinge aiming.
      // Total remains eighteen samples over all six faces, within the existing 96-step pass.
      if (face == Direction.UP)
        for (double x : new double[] {.25, .75})
          for (double z : new double[] {.25, .75})
            points.add(
                new Vec3(
                    support.getX() + box.minX + (box.maxX - box.minX) * x,
                    y,
                    support.getZ() + box.minZ + (box.maxZ - box.minZ) * z));
      return points;
    }
    for (double height : new double[] {.25, .75, .5}) {
      points.add(
          new Vec3(
              support.getX()
                  + (face.getStepX() == 0
                      ? (box.minX + box.maxX) / 2
                      : face.getStepX() > 0 ? box.maxX : box.minX),
              support.getY()
                  + (face.getStepY() == 0
                      ? box.minY + (box.maxY - box.minY) * height
                      : face.getStepY() > 0 ? box.maxY : box.minY),
              support.getZ()
                  + (face.getStepZ() == 0
                      ? (box.minZ + box.maxZ) / 2
                      : face.getStepZ() > 0 ? box.maxZ : box.minZ)));
    }
    return points;
  }

  public static Probe find(BlockPos at, BlockState wanted, ItemStack stack) {
    var mc = Minecraft.getInstance();
    var p = mc.player;
    if (!(stack.getItem() instanceof BlockItem item)
        || !SchematicMaterials.automaticItem(item)
        || !SchematicMaterials.automaticBlock(wanted)
        || !receivedArea(at)
        || !protectedNeighbours(at, wanted, Set.of(at))
        || !replaceable(at, wanted)
        || at.distToCenterSqr(p.getEyePosition()) > Math.pow(p.blockInteractionRange() + 1, 2))
      return null;
    if (stack.has(net.minecraft.core.component.DataComponents.CONTAINER)
        || stack.has(net.minecraft.core.component.DataComponents.CUSTOM_DATA)
        || stack.has(net.minecraft.core.component.DataComponents.BLOCK_STATE)
        || stack.has(net.minecraft.core.component.DataComponents.BLOCK_ENTITY_DATA)) return null;
    // Neighbour-sensitive interactive blocks are never used as support: no opening/toggling side
    // effects.
    float yaw = p.getYRot(), pitch = p.getXRot();
    try {
      for (Direction face : Direction.values()) {
        boolean accumulating = !mc.level.getBlockState(at).isAir();
        BlockPos support = accumulating ? at : at.relative(face.getOpposite());
        var supportState = mc.level.getBlockState(support);
        if (!SchematicConstruction.received(support)
            || supportState.isAir()
            || supportState.hasBlockEntity()
            || (!accumulating && !interactionFree(supportState))
            || (!accumulating && !supportState.isCollisionShapeFullBlock(mc.level, support))
            || supportState.getBlock() instanceof DoorBlock
            || supportState.getBlock() instanceof TrapDoorBlock
            || supportState.getBlock() instanceof FenceGateBlock) continue;
        var shape = supportState.getShape(mc.level, support);
        if (shape.isEmpty()) continue;
        var box = shape.bounds();
        for (var sample : supportPoints(support, box, face)) {
          if (++work > 96) return null;
          var point = sample;
          if (point.distanceTo(p.getEyePosition()) > p.blockInteractionRange()) continue;
          var ray =
              supportHit(
                  p.getEyePosition(),
                  point,
                  face,
                  support,
                  p.blockInteractionRange(),
                  end ->
                      mc.level.clip(
                          new ClipContext(
                              p.getEyePosition(),
                              end,
                              ClipContext.Block.OUTLINE,
                              ClipContext.Fluid.NONE,
                              p)));
          if (ray == null) continue;
          point = ray.getLocation();
          var delta = point.subtract(p.getEyePosition());
          boolean blocked = false;
          for (var entity :
              mc.level.getEntities(p, p.getBoundingBox().expandTowards(delta).inflate(1)))
            if (entity.isPickable()
                && !entity.isSpectator()
                && entity
                    .getBoundingBox()
                    .inflate(entity.getPickRadius())
                    .clip(p.getEyePosition(), point)
                    .isPresent()) {
              blocked = true;
              break;
            }
          if (blocked) continue;
          float y = (float) Math.toDegrees(Math.atan2(-delta.x, delta.z)),
              x = (float) -Math.toDegrees(Math.atan2(delta.y, Math.hypot(delta.x, delta.z)));
          p.setYRot(y);
          p.setXRot(x);
          var hit = ray;
          var context = new BlockPlaceContext(p, InteractionHand.MAIN_HAND, stack, hit);
          var updated = item.updatePlacementContext(context);
          if (updated == null || !updated.getClickedPos().equals(at) || !updated.canPlace())
            continue;
          var predicted =
              ((SchematicBlockItemAccessor) (Object) item).rusty$placementState(updated);
          if (predicted == null || !compatible(wanted, predicted)) continue;
          var writes = new LinkedHashMap<BlockPos, BlockState>();
          writes.put(at, predicted);
          if (predicted.getBlock() instanceof DoorBlock
              || predicted.getBlock() instanceof DoublePlantBlock) {
            if (!predicted.hasProperty(BlockStateProperties.DOUBLE_BLOCK_HALF)
                || predicted.getValue(BlockStateProperties.DOUBLE_BLOCK_HALF)
                    != DoubleBlockHalf.LOWER) continue;
            writes.put(
                at.above(),
                predicted.setValue(BlockStateProperties.DOUBLE_BLOCK_HALF, DoubleBlockHalf.UPPER));
          } else if (predicted.getBlock() instanceof AbstractBedBlock) {
            if (predicted.getValue(BedBlock.PART) != BedPart.FOOT) continue;
            writes.put(
                at.relative(predicted.getValue(BedBlock.FACING)),
                predicted.setValue(BedBlock.PART, BedPart.HEAD));
          }
          boolean allowed = true;
          for (var write : writes.entrySet()) {
            var target = RustySchematics.targets().get(write.getKey());
            if (target == null
                || !receivedArea(write.getKey())
                || !protectedNeighbours(write.getKey(), write.getValue(), writes.keySet())
                || !replaceable(write.getKey(), target)
                || !compatible(target, write.getValue())
                || !mc.level.isUnobstructed(
                    write.getValue(),
                    write.getKey(),
                    net.minecraft.world.phys.shapes.CollisionContext.placementContext(p))) {
              allowed = false;
              break;
            }
          }
          if (allowed) return new Probe(hit, Map.copyOf(writes), y, x);
        }
      }
    } finally {
      p.setYRot(yaw);
      p.setXRot(pitch);
    }
    return null;
  }

  private static boolean receivedArea(BlockPos at) {
    var mc = Minecraft.getInstance();
    if (!SchematicConstruction.received(at) || !mc.level.getWorldBorder().isWithinBounds(at))
      return false;
    for (Direction face : Direction.values())
      if (!SchematicConstruction.received(at.relative(face))) return false;
    return true;
  }
}
