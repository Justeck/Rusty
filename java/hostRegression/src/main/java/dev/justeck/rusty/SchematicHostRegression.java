package dev.justeck.rusty;

import dev.justeck.rusty.core.*;
import java.util.*;
import net.minecraft.SharedConstants;
import net.minecraft.core.*;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.*;
import net.minecraft.world.level.block.state.properties.*;
import net.minecraft.world.phys.*;

/** Actual host math/state/lifecycle regressions; no client/server/window/world is started. */
public final class SchematicHostRegression {
  private static int checks, failures;

  private static void check(String name, Runnable test) {
    try {
      test.run();
      checks++;
      System.out.println("PASS " + name);
    } catch (Throwable e) {
      failures++;
      System.out.println("FAIL " + name + ": " + e);
      e.printStackTrace(System.out);
    }
  }

  private static void require(boolean value, String reason) {
    if (!value) throw new AssertionError(reason);
  }

  private static Schematic.Document document(int x, List<Schematic.State> palette) {
    return new Schematic.Document(
        List.of(
            new Schematic.Cell(
                new Schematic.Pos(x, 0, 0), Schematic.State.parse("minecraft:stone"))),
        0,
        "Owned host regression",
        palette);
  }

  private static void origin(int x) {
    try {
      var field = RustySchematics.class.getDeclaredField("origin");
      field.setAccessible(true);
      field.set(null, new BlockPos(x, 200, 0));
    } catch (Exception e) {
      throw new RuntimeException(e);
    }
  }

  private static void validate() {
    RustySchematics.validationWork = 0;
    RustySchematics.advanceValidation();
  }

  private static void transform() {
    RustySchematics.transformWork = 0;
    RustySchematics.advanceTransform();
  }

  private static void accept(Schematic.Document d) {
    RustySchematics.clear();
    origin(1);
    RustySchematics.accept(d);
    validate();
    transform();
  }

  /** Read-only cells from the owned fixture; no game, world constructor, or packet is started. */
  private static final class FixtureLevel extends net.minecraft.client.multiplayer.ClientLevel {
    Map<BlockPos, BlockState> cells;

    private FixtureLevel() {
      super(null, null, null, null, 0, 0, null, false, 0, 0);
      throw new AssertionError("Fixture level constructor must never run");
    }

    @Override
    public BlockState getBlockState(BlockPos pos) {
      if (pos.getX() < -18
          || pos.getX() > 18
          || pos.getZ() < -18
          || pos.getZ() > 22
          || pos.getY() < 198
          || pos.getY() > 204)
        throw new AssertionError("Read outside known owned fixture box: " + pos);
      return cells.getOrDefault(pos, Blocks.AIR.defaultBlockState());
    }

    @Override
    public net.minecraft.world.level.material.FluidState getFluidState(BlockPos pos) {
      return getBlockState(pos).getFluidState();
    }

    @Override
    public int getMinY() {
      return -64;
    }

    @Override
    public int getHeight() {
      return 384;
    }

    @Override
    public boolean hasNeighborSignal(BlockPos pos) {
      return false;
    }

    @Override
    public boolean setBlock(BlockPos pos, BlockState state, int flags, int recursion) {
      throw new AssertionError("Read-only host fixture cannot write a block");
    }
  }

  private static FixtureLevel placementFixture() {
    try {
      var field = sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
      field.setAccessible(true);
      var level =
          (FixtureLevel) ((sun.misc.Unsafe) field.get(null)).allocateInstance(FixtureLevel.class);
      level.cells = new HashMap<>();
      for (int x = -18; x <= 18; x++)
        for (int z = -18; z <= 22; z++)
          for (int y = 198; y <= 199; y++)
            level.cells.put(new BlockPos(x, y, z), Blocks.STONE.defaultBlockState());
      level.cells.put(new BlockPos(3, 202, 11), Blocks.STONE.defaultBlockState());
      level.cells.put(new BlockPos(0, 200, 3), Blocks.COBBLESTONE.defaultBlockState());
      level.cells.put(new BlockPos(1, 200, 3), Blocks.OAK_PLANKS.defaultBlockState());
      level.cells.put(new BlockPos(0, 200, 4), Blocks.GLASS.defaultBlockState());
      level.cells.put(
          new BlockPos(1, 200, 4),
          Blocks.OAK_LOG.defaultBlockState().setValue(RotatedPillarBlock.AXIS, Direction.Axis.X));
      level.cells.put(new BlockPos(0, 200, 6), Blocks.OAK_SLAB.defaultBlockState());
      level.cells.put(new BlockPos(0, 200, 9), Blocks.COBBLESTONE.defaultBlockState());
      level.cells.put(new BlockPos(0, 201, 9), Blocks.OAK_PLANKS.defaultBlockState());
      var bed =
          net.minecraft.core.registries.BuiltInRegistries.BLOCK
              .getValue(net.minecraft.resources.Identifier.parse("minecraft:red_bed"))
              .defaultBlockState()
              .setValue(AbstractBedBlock.FACING, Direction.SOUTH);
      level.cells.put(new BlockPos(2, 200, 9), bed.setValue(AbstractBedBlock.PART, BedPart.FOOT));
      level.cells.put(new BlockPos(2, 200, 10), bed.setValue(AbstractBedBlock.PART, BedPart.HEAD));
      level.cells = Map.copyOf(level.cells);
      return level;
    } catch (ReflectiveOperationException e) {
      throw new RuntimeException(e);
    }
  }

  private static BlockState doorAt(FixtureLevel level, Vec3 eye, Vec3 point) {
    var at = new BlockPos(1, 200, 9);
    var hit =
        SchematicPlacement.supportHit(
            eye,
            point,
            Direction.UP,
            at.below(),
            4.5,
            end ->
                level.clip(
                    new net.minecraft.world.level.ClipContext(
                        eye,
                        end,
                        net.minecraft.world.level.ClipContext.Block.OUTLINE,
                        net.minecraft.world.level.ClipContext.Fluid.NONE,
                        net.minecraft.world.phys.shapes.CollisionContext.empty())));
    if (hit == null) return null;
    var delta = hit.getLocation().subtract(eye);
    float yaw = (float) Math.toDegrees(Math.atan2(-delta.x, delta.z));
    // DoorBlock's state/canSurvive queries use geometry and direction only; no item components,
    // player actions, synthetic placement receipts, or block mutation are involved here.
    var context =
        new net.minecraft.world.item.context.BlockPlaceContext(
            level,
            null,
            net.minecraft.world.InteractionHand.MAIN_HAND,
            net.minecraft.world.item.ItemStack.EMPTY,
            hit) {
          @Override
          public Direction getHorizontalDirection() {
            return Direction.fromYRot(yaw);
          }
        };
    var state = Blocks.OAK_DOOR.getStateForPlacement(context);
    return context.getClickedPos().equals(at) && state != null && state.canSurvive(level, at)
        ? state
        : null;
  }

  private static net.minecraft.world.item.ItemStack identityStack(
      net.minecraft.world.item.Item item) {
    // Item identity/count is sufficient for canBeReplaced. This host-only fixture has no runtime
    // component registry, item action, or receipt; construct its explicit empty component map.
    try {
      var constructor =
          net.minecraft.world.item.ItemStack.class.getDeclaredConstructor(
              Holder.class, int.class, net.minecraft.core.component.PatchedDataComponentMap.class);
      constructor.setAccessible(true);
      return constructor.newInstance(
          item.builtInRegistryHolder(),
          2,
          new net.minecraft.core.component.PatchedDataComponentMap(
              net.minecraft.core.component.DataComponentMap.EMPTY));
    } catch (ReflectiveOperationException e) {
      throw new RuntimeException(e);
    }
  }

  public static void main(String[] args) {
    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    check(
        "Survival/Creative and mayBuild are independent required host capabilities",
        SchematicHostRegression::buildEligibility);
    var stone = Schematic.State.parse("minecraft:stone");
    check(
        "received damage releases every construction phase without health loss",
        () -> damageLifecycle(false));
    check(
        "received damage preserves a manually selected slot and never rearms",
        () -> damageLifecycle(true));
    check(
        "owned damage fixture supplies a valid single-player command target",
        () -> {
          String command = dev.justeck.rusty.qa.SchematicAcceptance.damageCommand();
          try {
            var selector =
                net.minecraft.commands.arguments.EntityArgument.entity()
                    .parse(new com.mojang.brigadier.StringReader(command.split(" ")[1]));
            require(selector.getMaxResults() == 1, "Damage must target one owned player");
            require(!selector.includesEntities(), "Damage target must only select players");
          } catch (com.mojang.brigadier.exceptions.CommandSyntaxException e) {
            throw new AssertionError(
                "Fixture damage command is rejected before applying damage", e);
          }
        });
    check(
        "diagnose ordinary door floor center versus off-center from safe diagonal",
        () -> {
          var level = placementFixture();
          var eye = new Vec3(.5, 201.62, 8.5);
          var center = doorAt(level, eye, new Vec3(1.5, 200, 9.5));
          var offset = doorAt(level, eye, new Vec3(1.75, 200, 9.25));
          require(
              center == null, "Legacy center-only floor ray is occluded by the owned west column");
          require(
              offset != null
                  && offset.getValue(DoorBlock.FACING) == Direction.EAST
                  && offset.getValue(DoorBlock.HINGE) == DoorHingeSide.LEFT,
              "Off-center ordinary floor hit must prove exact EAST/LEFT placement feasible");
          var grid = new Mining.Grid(new Mining.Pos(-3, 198, 3), 8, 7, 10);
          for (int i = 0; i < grid.size(); i++) {
            var p = grid.position(i);
            var at = new BlockPos(p.x(), p.y(), p.z());
            var state = level.getBlockState(at);
            grid.set(
                p,
                state.isAir()
                    ? Mining.AIR
                    : Mining.passive(state.isCollisionShapeFullBlock(level, at)));
          }
          int[][] path = {
            {0, 200, 5}, {-1, 200, 5}, {-1, 200, 6}, {-1, 200, 7}, {0, 200, 7}, {0, 200, 8}
          };
          for (int i = 1; i < path.length; i++) {
            var a = path[i - 1];
            var b = path[i];
            require(
                grid.canWalk(new Mining.Pos(a[0], a[1], a[2]), new Mining.Pos(b[0], b[1], b[2])),
                "Exact diagonal is accessible by unchanged ordinary safe floor edges");
          }
        });
    check(
        "dry slab neighbour keeps exact state when ordinary north stairs are added",
        () -> {
          var slab = Blocks.OAK_SLAB.defaultBlockState();
          var stairs =
              Blocks.OAK_STAIRS.defaultBlockState().setValue(StairBlock.FACING, Direction.NORTH);
          for (Direction direction : Direction.values()) {
            var updated =
                slab.updateShape(
                    null,
                    null,
                    new BlockPos(0, 200, 6),
                    direction,
                    new BlockPos(1, 200, 6),
                    stairs,
                    net.minecraft.util.RandomSource.create(1));
            require(updated.equals(slab), "Actual dry slab callback preserves exact state");
          }
          require(
              SchematicPlacement.stableNeighbour(slab, stairs, Direction.EAST, true),
              "Dry shape-preserving slab must not prevent the adjacent stairs");
          require(
              !SchematicPlacement.stableNeighbour(
                  slab.setValue(SlabBlock.WATERLOGGED, true), stairs, Direction.EAST, true),
              "Waterlogged slab remains protected against scheduled fluid work");
        });
    check(
        "ordinary support sampling discovers exact EAST LEFT door from accessible diagonal",
        () -> {
          var level = placementFixture();
          var eye = new Vec3(.5, 201.62, 8.5);
          var wanted =
              Blocks.OAK_DOOR
                  .defaultBlockState()
                  .setValue(DoorBlock.FACING, Direction.EAST)
                  .setValue(DoorBlock.HINGE, DoorHingeSide.LEFT);
          boolean found = false;
          for (var point :
              SchematicPlacement.supportPoints(
                  new BlockPos(1, 199, 9), new AABB(0, 0, 0, 1, 1, 1), Direction.UP))
            if (wanted.equals(doorAt(level, eye, point))) found = true;
          require(
              found,
              "Production support samples must include a feasible exact EAST/LEFT floor hit");
          var budget = 0;
          for (Direction face : Direction.values())
            budget +=
                SchematicPlacement.supportPoints(
                        new BlockPos(1, 199, 9), new AABB(0, 0, 0, 1, 1, 1), face)
                    .size();
          require(
              budget * 5 <= 96,
              "Four bounded candidates and one selected-stack re-probe still fit the existing"
                  + " 96-step allowance");
        });
    check(
        "partial slab and snow outlines retain ordinary top hits and exact accumulation",
        () -> {
          var level = placementFixture();
          var at = new BlockPos(8, 200, 0);
          var eye = new Vec3(7.5, 201.62, -3.5);
          for (var current :
              List.of(
                  Blocks.OAK_SLAB.defaultBlockState(),
                  Blocks.SNOW.defaultBlockState(),
                  Blocks.SNOW.defaultBlockState().setValue(SnowLayerBlock.LAYERS, 2),
                  Blocks.SNOW.defaultBlockState().setValue(SnowLayerBlock.LAYERS, 3))) {
            var cells = new HashMap<>(level.cells);
            cells.put(at, current);
            level.cells = Map.copyOf(cells);
            var wanted =
                current.is(Blocks.OAK_SLAB)
                    ? current.setValue(SlabBlock.TYPE, SlabType.DOUBLE)
                    : current.setValue(
                        SnowLayerBlock.LAYERS, current.getValue(SnowLayerBlock.LAYERS) + 1);
            var stack = identityStack(current.getBlock().asItem());
            var shape = current.getShape(level, at);
            boolean found = false;
            for (var point : SchematicPlacement.supportPoints(at, shape.bounds(), Direction.UP)) {
              var hit =
                  SchematicPlacement.supportHit(
                      eye,
                      point,
                      Direction.UP,
                      at,
                      4.5,
                      end ->
                          level.clip(
                              new net.minecraft.world.level.ClipContext(
                                  eye,
                                  end,
                                  net.minecraft.world.level.ClipContext.Block.OUTLINE,
                                  net.minecraft.world.level.ClipContext.Fluid.NONE,
                                  net.minecraft.world.phys.shapes.CollisionContext.empty())));
              if (hit == null) continue;
              var context =
                  new net.minecraft.world.item.context.BlockPlaceContext(
                      level, null, net.minecraft.world.InteractionHand.MAIN_HAND, stack, hit) {};
              var predicted = current.getBlock().getStateForPlacement(context);
              require(
                  hit.getLocation().y == at.getY() + shape.bounds().maxY,
                  "Use actual partial outline surface, never a cube-top or fabricated hit");
              if (context.getClickedPos().equals(at)
                  && context.canPlace()
                  && wanted.equals(predicted)
                  && predicted.canSurvive(level, at)
                  && SchematicPlacement.compatible(wanted, predicted)) found = true;
            }
            require(
                found,
                "Ordinary actual-shape top hit must accumulate " + current + " to " + wanted);
          }
        });
    check(
        "ordinary support-face hit is inside ray segment and returns real surface hit",
        () -> {
          var eye = new Vec3(.5, 201.62, .5);
          var surface = new Vec3(.5, 200, 3.5);
          var at = new BlockPos(0, 199, 3);
          var boxes = List.of(new AABB(0, 0, 0, 1, 1, 1));
          require(
              AABB.clip(boxes, eye, surface, at) == null,
              "Host endpoint contact must reproduce initial MISS");
          var hit =
              SchematicPlacement.supportHit(
                  eye, surface, Direction.UP, at, 4.5, to -> AABB.clip(boxes, eye, to, at));
          require(
              hit != null && hit.getDirection() == Direction.UP, "Ordinary support must be hit");
          require(
              hit.getLocation().y == 200 && hit.getLocation().z < surface.z,
              "Use actual surface hit rather than fabricated endpoint");
        });
    check(
        "support ray rejects occluder and preserves ordinary reach",
        () -> {
          var eye = new Vec3(.5, 201.62, .5);
          var surface = new Vec3(.5, 200, 3.5);
          var at = new BlockPos(0, 199, 3);
          require(
              SchematicPlacement.supportHit(
                      eye,
                      surface,
                      Direction.UP,
                      at,
                      4.5,
                      to ->
                          new BlockHitResult(
                              new Vec3(.5, 200, 1.5), Direction.UP, new BlockPos(0, 199, 1), false))
                  == null,
              "Occluding support cannot authorize target");
          require(
              SchematicPlacement.supportHit(
                      eye,
                      surface,
                      Direction.UP,
                      at,
                      2,
                      to -> AABB.clip(List.of(new AABB(0, 0, 0, 1, 1, 1)), eye, to, at))
                  == null,
              "No reach extension");
        });
    check(
        "planned reactive chest neighbor cannot authorize undeclared synchronous LEFT write",
        () -> {
          var single =
              Blocks.CHEST
                  .defaultBlockState()
                  .setValue(ChestBlock.FACING, Direction.NORTH)
                  .setValue(ChestBlock.TYPE, ChestType.SINGLE);
          var right = single.setValue(ChestBlock.TYPE, ChestType.RIGHT);
          var changed =
              single.updateShape(
                  null,
                  null,
                  new BlockPos(0, 200, 0),
                  Direction.EAST,
                  new BlockPos(1, 200, 0),
                  right,
                  net.minecraft.util.RandomSource.create(1));
          require(
              changed.getValue(ChestBlock.TYPE) == ChestType.LEFT,
              "Actual host callback must establish synchronous mutation");
          require(
              !SchematicPlacement.stableNeighbour(single, right, Direction.EAST, true),
              "Planned SINGLE chest must stay protected");
          require(single.getValue(ChestBlock.TYPE) == ChestType.SINGLE, "No world-mutating probe");
        });
    check(
        "cancel after one registry batch retains preceding accepted document",
        () -> {
          var old = document(0, List.of(stone));
          accept(old);
          var states =
              net.minecraft.core.registries.BuiltInRegistries.BLOCK.stream()
                  .flatMap(b -> b.getStateDefinition().getPossibleStates().stream())
                  .limit(1200)
                  .map(RustySchematics::describe)
                  .toList();
          require(states.size() > 512, "Real host palette spans validation batches");
          var next = document(1, states);
          require(
              RustySchematics.document == old && RustySchematics.ready(),
              "Prior document genuinely activated");
          RustySchematics.accept(next);
          validate();
          require(
              RustySchematics.validationWork >= 512 && RustySchematics.document == old,
              "Exactly one partial validation batch before cancellation");
          SchematicFiles.cancel();
          for (int i = 0; i < 5; i++) validate();
          require(
              RustySchematics.document == old,
              "Cancelled pending import must never activate later");
        });
    check(
        "overflow transform is not ready and leaves no preview",
        () -> {
          accept(
              new Schematic.Document(
                  List.of(
                      new Schematic.Cell(new Schematic.Pos(0, 0, 0), stone),
                      new Schematic.Cell(new Schematic.Pos(Integer.MAX_VALUE, 0, 0), stone)),
                  0,
                  "Partial projection overflow"));
          require(!RustySchematics.ready(), "Failed transform cannot start or complete");
          require(RustySchematics.targets().isEmpty(), "No partial target map");
          require(RustyPlanner.cells.isEmpty(), "No stale rendered preview");
        });
    check(
        "unused unknown palette state rejects before activation",
        () -> {
          var old = document(0, List.of(stone));
          accept(old);
          RustySchematics.accept(
              document(1, List.of(stone, Schematic.State.parse("unknown:unused"))));
          validate();
          require(
              RustySchematics.document == old && RustyClient.notice.contains("Unknown block"),
              "Full declared palette is host validated transactionally");
        });
    check(
        "successful all-air transform remains a legitimate empty ready plan",
        () -> {
          var air = Schematic.State.parse("minecraft:air");
          accept(
              new Schematic.Document(
                  List.of(new Schematic.Cell(new Schematic.Pos(0, 0, 0), air)),
                  0,
                  "Owned all-air regression"));
          require(
              RustySchematics.ready() && RustySchematics.targets().isEmpty(),
              "Successful all-air plan stays distinct from transform failure");
        });
    check(
        "benign coupled neighbours remain supported while power-changing door relations are"
            + " refused",
        () -> {
          var door = Blocks.OAK_DOOR.defaultBlockState();
          var bed =
              net.minecraft.core.registries.BuiltInRegistries.BLOCK
                  .getValue(net.minecraft.resources.Identifier.parse("minecraft:red_bed"))
                  .defaultBlockState();
          require(
              SchematicPlacement.stableNeighbour(door, bed, Direction.EAST, true),
              "Required bed beside door relation is unchanged");
          require(
              !SchematicPlacement.stableNeighbour(
                  door, Blocks.REDSTONE_BLOCK.defaultBlockState(), Direction.EAST, true),
              "New signal source may not silently power planned door");
          require(
              SchematicPlacement.stableNeighbour(
                  bed, Blocks.STONE.defaultBlockState(), Direction.WEST, true),
              "Side of bed is not its coupled partner");
          require(
              !SchematicPlacement.stableNeighbour(
                  bed,
                  Blocks.STONE.defaultBlockState(),
                  AbstractBedBlock.getConnectedDirection(bed),
                  true),
              "Coupled bed partner relation remains protected");
        });
    check(
        "ordinary one-block ascent releases jump before landing and cannot request a rebound",
        () -> {
          require(
              SchematicConstruction.ascentJump(200, 201, 200),
              "Initial one-block ascent requests ordinary jump");
          require(
              SchematicConstruction.ascentJump(200, 201, 200.49),
              "Early ascent still needs height");
          require(
              !SchematicConstruction.ascentJump(200, 201, 200.6),
              "Release jump once current feet height clears the ascent trigger");
          require(
              !SchematicConstruction.ascentJump(200, 201, 201),
              "Landing/alignment must not request another jump");
          require(
              !SchematicConstruction.ascentJump(200, 201, 202.25220334025371),
              "Observed rebound height must never keep jump held");
          require(
              !SchematicConstruction.ascentJump(200, 200, 200)
                  && !SchematicConstruction.ascentJump(200, 199, 200),
              "Flat/descent routes never request jump");
          var grid =
              new dev.justeck.rusty.core.Mining.Grid(
                  new dev.justeck.rusty.core.Mining.Pos(-3, 197, 0), 9, 8, 10);
          for (int i = 0; i < grid.size(); i++) {
            var p = grid.position(i);
            grid.set(
                p,
                p.y() <= 199
                    ? dev.justeck.rusty.core.Mining.WALL
                    : dev.justeck.rusty.core.Mining.AIR);
          }
          for (int[] xyz : new int[][] {{0, 200, 3}, {1, 200, 3}, {0, 200, 4}, {1, 200, 4}})
            grid.set(
                new dev.justeck.rusty.core.Mining.Pos(xyz[0], xyz[1], xyz[2]),
                dev.justeck.rusty.core.Mining.WALL);
          grid.set(
              new dev.justeck.rusty.core.Mining.Pos(0, 200, 6),
              dev.justeck.rusty.core.Mining.passive(false));
          var top = new dev.justeck.rusty.core.Mining.Pos(1, 201, 4);
          require(
              grid.canWalk(new dev.justeck.rusty.core.Mining.Pos(2, 200, 4), top)
                  && grid.canWalk(new dev.justeck.rusty.core.Mining.Pos(1, 200, 5), top),
              "Both unchanged received floor-to-log-top edges remain safe");
          var observed = new Vec3(1.2525217498466987, 202.25220334025371, 4.341041292168393);
          require(
              observed.distanceToSqr(new Vec3(2.5, 200, 4.5)) > 6
                  && observed.distanceToSqr(new Vec3(1.5, 200, 5.5)) > 6,
              "Recorded rebound really exceeds existing safety radius; do not loosen that guard");
        });
    check(
        "stale ordinary NORTH look cannot authorize SOUTH bed with a different coupled footprint",
        () -> {
          var look = new SchematicLook();
          var context =
              new SchematicLook.Context(
                  new Object(), new Object(), new Object(), "owned/overworld");
          var pose = new SchematicLook.Pose(1.506988589686281, 200, 5.512360487151354);
          var foot = new BlockPos(2, 200, 9);
          var bed =
              net.minecraft.core.registries.BuiltInRegistries.BLOCK
                  .getValue(net.minecraft.resources.Identifier.parse("minecraft:red_bed"))
                  .defaultBlockState();
          var wanted = bed.setValue(AbstractBedBlock.FACING, Direction.fromYRot(0));
          var stale = bed.setValue(AbstractBedBlock.FACING, Direction.fromYRot(180));
          require(
              foot.relative(wanted.getValue(AbstractBedBlock.FACING))
                      .equals(new BlockPos(2, 200, 10))
                  && foot.relative(stale.getValue(AbstractBedBlock.FACING))
                      .equals(new BlockPos(2, 200, 8)),
              "Host direction defines the exact observed intended/outside head footprints");
          look.observe(context, 180, 20);
          var staged = look.aim(context, pose, 0, 20, 0);
          require(
              look.check(staged, context, pose, 1) == SchematicLook.Gate.WAITING,
              "Stale server heading cannot authorize any bed write");
          look.observe(context, 0, 20);
          require(
              look.check(staged, context, pose, 1) == SchematicLook.Gate.READY,
              "Only the ordinary matching look authorizes revalidated intended footprint");
        });
    System.out.println("Host regressions: " + checks + " passed, " + failures + " failed");
    if (failures > 0) throw new AssertionError("Host regression failures: " + failures);
  }

  private static void buildEligibility() {
    try {
      var f = sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
      f.setAccessible(true);
      var unsafe = (sun.misc.Unsafe) f.get(null);
      var instance = net.minecraft.client.Minecraft.class.getDeclaredField("instance");
      instance.setAccessible(true);
      var old = instance.get(null);
      try {
        var mc =
            (net.minecraft.client.Minecraft)
                unsafe.allocateInstance(net.minecraft.client.Minecraft.class);
        mc.player =
            (net.minecraft.client.player.LocalPlayer)
                unsafe.allocateInstance(net.minecraft.client.player.LocalPlayer.class);
        mc.gameMode =
            (net.minecraft.client.multiplayer.MultiPlayerGameMode)
                unsafe.allocateInstance(net.minecraft.client.multiplayer.MultiPlayerGameMode.class);
        var abilities = new net.minecraft.world.entity.player.Abilities();
        var field = net.minecraft.world.entity.player.Player.class.getDeclaredField("abilities");
        field.setAccessible(true);
        field.set(mc.player, abilities);
        var mode = mc.gameMode.getClass().getDeclaredField("localPlayerMode");
        mode.setAccessible(true);
        instance.set(null, mc);
        for (var gameType : net.minecraft.world.level.GameType.values())
          for (boolean mayBuild : new boolean[] {false, true}) {
            mode.set(mc.gameMode, gameType);
            abilities.mayBuild = mayBuild;
            boolean eligible =
                mayBuild
                    && (gameType == net.minecraft.world.level.GameType.SURVIVAL
                        || gameType == net.minecraft.world.level.GameType.CREATIVE);
            require(
                SchematicConstruction.buildEligible() == eligible,
                "Host mode/ability boundary " + gameType + "/" + mayBuild);
          }
      } finally {
        instance.set(null, old);
      }
    } catch (ReflectiveOperationException e) {
      throw new RuntimeException(e);
    }
  }

  private static void damageLifecycle(boolean manualSlot) {
    // Only allocate the minimal host objects used by the real stop lifecycle. No constructor,
    // game loop, window, network or item action is started; the actual game receipt test is
    // separate.
    try {
      var unsafeField = sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
      unsafeField.setAccessible(true);
      var unsafe = (sun.misc.Unsafe) unsafeField.get(null);
      var instanceField = net.minecraft.client.Minecraft.class.getDeclaredField("instance");
      instanceField.setAccessible(true);
      Object oldMinecraft = instanceField.get(null);
      var runField = SchematicConstruction.class.getDeclaredField("run");
      runField.setAccessible(true);
      Object oldRun = runField.get(null);
      try {
        var client =
            (net.minecraft.client.Minecraft)
                unsafe.allocateInstance(net.minecraft.client.Minecraft.class);
        var player =
            (net.minecraft.client.player.LocalPlayer)
                unsafe.allocateInstance(net.minecraft.client.player.LocalPlayer.class);
        client.player = player;
        instanceField.set(null, client);
        var inventory =
            new net.minecraft.world.entity.player.Inventory(
                player, new net.minecraft.world.entity.EntityEquipment());
        var inventoryField =
            net.minecraft.world.entity.player.Player.class.getDeclaredField("inventory");
        inventoryField.setAccessible(true);
        inventoryField.set(player, inventory);
        var runType = runField.getType();
        var constructor = runType.getDeclaredConstructor();
        constructor.setAccessible(true);
        for (int phase : new int[] {0, 1, 2, 3}) {
          var run = constructor.newInstance();
          Object listener = new Object(), level = new Object();
          String scope = "owned-damage-test";
          var context = new SchematicLook.Context(player, listener, level, scope);
          var pose = new SchematicLook.Pose(1, 200, 1);
          var look = new SchematicLook();
          var aim = look.aim(context, pose, 0, 20, 0);
          Map<String, Object> fields = new LinkedHashMap<>();
          fields.put("player", player);
          fields.put("connection", listener);
          fields.put("level", level);
          fields.put("scope", scope);
          fields.put("phase", phase);
          fields.put(
              "cells",
              java.util.Collections.nCopies(
                  4096, Map.entry(new BlockPos(0, 220, 0), Blocks.STONE.defaultBlockState())));
          var counts = new SchematicSurvey.Counts();
          for (int i = 0; i < 2048; i++) counts.observe(1, false, false, false);
          fields.put("counts", counts);
          fields.put("health", 20f);
          fields.put("ownedSlot", 0);
          fields.put("previousSlot", 4);
          fields.put("step", new BlockPos(1, 200, 2));
          fields.put("aim", aim);
          fields.put("transfer", new SchematicSafety.Transfer<>(9, 36, "stone", "tool"));
          fields.put(
              "pending",
              new SchematicPlacement.Probe(
                  new BlockHitResult(
                      new Vec3(1.5, 200, 3.5), Direction.UP, new BlockPos(1, 199, 3), false),
                  Map.of(new BlockPos(1, 200, 3), Blocks.STONE.defaultBlockState()),
                  0,
                  20));
          for (var entry : fields.entrySet()) {
            var field = runType.getDeclaredField(entry.getKey());
            field.setAccessible(true);
            field.set(run, entry.getValue());
          }
          inventory.setSelectedSlot(manualSlot ? 2 : 0);
          runField.set(null, run);
          SchematicConstruction.placed = 0;
          SchematicConstruction.damageReceipt(new Object(), player, level, scope);
          SchematicConstruction.damageReceipt(listener, new Object(), level, scope);
          SchematicConstruction.damageReceipt(listener, player, new Object(), scope);
          SchematicConstruction.damageReceipt(listener, player, level, "old-scope");
          require(
              SchematicConstruction.active(),
              "Other entity, listener, world and scope cannot stop owned run");
          SchematicConstruction.damageReceipt(listener, player, level, scope);
          require(
              !SchematicConstruction.active() && !SchematicConstruction.walking(),
              "Real local damage must release run and walking even with unchanged health");
          require(
              SchematicConstruction.result.startsWith("Damage"), "Damage remains an explicit stop");
          var outcome = SchematicConstruction.outcome;
          require(
              outcome.initiallyMatching() == null
                  && !outcome.surveyComplete()
                  && outcome.finallyMatching() == 2048
                  && outcome.unknown() == 2048
                  && outcome.receiptPending(),
              "Mid-survey stop retains known counts and explicit unscanned/pending uncertainty");
          require(
              look.check(aim, context, pose, 1) == SchematicLook.Gate.CANCELLED,
              "Pending aim is cancelled");
          require(
              inventory.getSelectedSlot() == (manualSlot ? 2 : 4),
              "Release only the still-owned slot");
          SchematicConstruction.damageReceipt(listener, player, level, scope);
          SchematicConstruction.tick(true);
          require(
              !SchematicConstruction.active() && SchematicConstruction.placed == 0,
              "Later receipt/tick cannot rearm or place");
        }
      } finally {
        runField.set(null, oldRun);
        instanceField.set(null, oldMinecraft);
      }
    } catch (ReflectiveOperationException e) {
      throw new RuntimeException(e);
    }
  }
}
