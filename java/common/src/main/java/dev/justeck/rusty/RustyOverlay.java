package dev.justeck.rusty;

import dev.justeck.rusty.core.OverlayBuffer;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.gizmos.*;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.LightLayer;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.phys.*;

/** Capture world facts on the game thread; rendering owns only immutable geometry snapshots. */
public final class RustyOverlay {
  private record Draw(Vec3 a, Vec3 b, int color, boolean box, boolean through) {}

  private static volatile List<Draw> snapshot = List.of();
  public static volatile int renderedGeometry;
  public static String trajectoryStatus = "Hold a bow or supported throwable.";

  private static void box(OverlayBuffer<Draw> out, AABB b, int color, boolean through) {
    out.add(
        new Draw(
            new Vec3(b.minX, b.minY, b.minZ),
            new Vec3(b.maxX, b.maxY, b.maxZ),
            color,
            true,
            through));
  }

  private static void line(OverlayBuffer<Draw> out, Vec3 a, Vec3 b, int color, boolean through) {
    out.add(new Draw(a, b, color, false, through));
  }

  public static void capture() {
    var mc = Minecraft.getInstance();
    if (mc.player == null || mc.level == null) {
      snapshot = List.of();
      return;
    }
    var out = new OverlayBuffer<Draw>();
    var cfg = RustyClient.config;
    Vec3 eye = RustyClient.active("freecam") ? RustyCamera.position : mc.player.getEyePosition();
    Vec3 start =
        eye.add(Vec3.directionFromRotation(mc.player.getXRot(), mc.player.getYRot()).scale(.25));
    // Target geometry belongs to Finder; X-ray only changes terrain rendering.
    if (RustyClient.active("finder")) {
      var targets =
          RustyClient.targets.keySet().stream()
              .map(BlockPos::of)
              .sorted(Comparator.comparingDouble(p -> p.distToCenterSqr(eye)))
              .limit(256)
              .toList();
      int tracer = 0;
      for (BlockPos p : targets) {
        if (cfg.outlines) box(out, new AABB(p).inflate(.003), 0xff59d8cd, true);
        if (cfg.tracers && tracer++ < cfg.tracerLimit)
          line(out, start, Vec3.atCenterOf(p), 0xcc62c7bd, true);
      }
    }
    if (RustyClient.active("entity_esp") || RustyClient.active("entity_tracers")) {
      int count = 0;
      for (var entity : mc.level.entitiesForRendering()) {
        if (++count > 512) break;
        if (entity == mc.player
            || entity.isRemoved()
            || entity.distanceToSqr(mc.player) > (double) cfg.entityRange * cfg.entityRange)
          continue;
        boolean player = entity instanceof Player;
        boolean living = entity instanceof LivingEntity, item = entity instanceof ItemEntity;
        int color = player ? 0xffdda167 : 0xff7dc8ab;
        if (RustyClient.active("entity_esp") && cfg.entitySelected(false, player, living, item))
          box(out, entity.getBoundingBox().inflate(.03), color, true);
        if (RustyClient.active("entity_tracers") && cfg.entitySelected(true, player, living, item))
          out.addEntityTracer(
              new Draw(start, entity.getBoundingBox().getCenter(), color, false, true));
      }
    }
    if (RustyClient.active("storage_esp")) {
      var center = mc.player.chunkPosition();
      int count = 0, inspected = 0;
      storage:
      for (int x = center.x() - 2; x <= center.x() + 2; x++)
        for (int z = center.z() - 2; z <= center.z() + 2; z++) {
          var chunk = mc.level.getChunkSource().getChunk(x, z, ChunkStatus.FULL, false);
          if (chunk == null) continue;
          for (var be : chunk.getBlockEntities().values()) {
            if (count >= 128 || inspected++ >= 512) break storage;
            String id = BuiltInRegistries.BLOCK_ENTITY_TYPE.getKey(be.getType()).getPath();
            if (id.contains("chest")
                || id.contains("shulker")
                || id.contains("furnace")
                || Set.of("barrel", "hopper", "dispenser", "dropper", "brewing_stand")
                    .contains(id)) {
              box(out, new AABB(be.getBlockPos()).inflate(.003), 0xffe2b869, true);
              count++;
            }
          }
        }
    }
    if (RustyClient.active("chunk_borders")) {
      var c = mc.player.chunkPosition();
      double x = c.x() * 16, z = c.z() * 16, min = mc.level.getMinY(), max = mc.level.getMaxY() + 1;
      for (int side = 0; side < 4; side++) {
        double xx = x + (side % 2) * 16, zz = z + (side / 2) * 16;
        line(out, new Vec3(xx, min, zz), new Vec3(xx, max, zz), 0xffd9bd67, false);
      }
      for (double y = min; y <= max; y += 16) {
        line(out, new Vec3(x, y, z), new Vec3(x + 16, y, z), 0x998fbc9a, false);
        line(out, new Vec3(x, y, z + 16), new Vec3(x + 16, y, z + 16), 0x998fbc9a, false);
        line(out, new Vec3(x, y, z), new Vec3(x, y, z + 16), 0x998fbc9a, false);
        line(out, new Vec3(x + 16, y, z), new Vec3(x + 16, y, z + 16), 0x998fbc9a, false);
      }
    }
    if (RustyClient.active("breadcrumbs")) {
      Vec3 previous = null;
      for (Vec3 p : RustyClient.trail) {
        if (previous != null && previous.distanceToSqr(p) < 256)
          line(out, previous.add(0, .05, 0), p.add(0, .05, 0), 0xffbc9259, false);
        previous = p;
      }
    }
    if (RustyClient.active("waypoints") || RustyClient.active("death_markers"))
      for (var marker : RustyNavigation.markers()) {
        if (marker.death() && !RustyClient.active("death_markers")) continue;
        box(
            out,
            new AABB(
                marker.x() - .3,
                marker.y(),
                marker.z() - .3,
                marker.x() + .3,
                marker.y() + 1,
                marker.z() + .3),
            marker.death() ? 0xffcf7164 : 0xff83b8a6,
            true);
      }
    if (RustyClient.active("build_planner"))
      for (var c : RustyPlanner.cells) box(out, new AABB(c.pos()).deflate(.005), c.color(), false);
    if (RustyClient.active("light_overlay")) {
      var p = mc.player.blockPosition();
      int sampled = 0;
      for (int x = -8; x <= 8; x += 2)
        for (int z = -8; z <= 8; z += 2) {
          BlockPos foot = p.offset(x, 0, z);
          if (mc.level
                  .getChunkSource()
                  .getChunk(foot.getX() >> 4, foot.getZ() >> 4, ChunkStatus.FULL, false)
              == null) continue;
          if (!mc.level
              .getChunkSource()
              .getLightEngine()
              .lightOnInColumn(
                  net.minecraft.world.level.ChunkPos.pack(foot.getX() >> 4, foot.getZ() >> 4)))
            continue;
          for (int y = 1; y >= -4; y--) {
            BlockPos floor = foot.offset(0, y - 1, 0);
            var state = mc.level.getBlockState(floor);
            if (!state.isSolidRender()
                || !mc.level
                    .getBlockState(floor.above())
                    .getCollisionShape(mc.level, floor.above())
                    .isEmpty()
                || !mc.level
                    .getBlockState(floor.above(2))
                    .getCollisionShape(mc.level, floor.above(2))
                    .isEmpty()) continue;
            int block = mc.level.getBrightness(LightLayer.BLOCK, floor.above()),
                sky = mc.level.getBrightness(LightLayer.SKY, floor.above());
            if (block == 0) {
              int color = sky > 0 ? 0xffd7ae55 : 0xffcd6860;
              Vec3 center = Vec3.atBottomCenterOf(floor.above()).add(0, .02, 0);
              line(out, center.add(-.25, 0, -.25), center.add(.25, 0, .25), color, false);
              line(out, center.add(-.25, 0, .25), center.add(.25, 0, -.25), color, false);
            }
            sampled++;
            break;
          }
        }
    }
    if (RustyClient.active("trajectories")) trajectory(out);
    snapshot = out.snapshot();
  }

  private static void trajectory(OverlayBuffer<Draw> out) {
    var mc = Minecraft.getInstance();
    var p = mc.player;
    var stack = p.getMainHandItem();
    double velocity, gravity;
    if (stack.is(Items.BOW)) {
      if (!p.isUsingItem()) {
        trajectoryStatus = "Draw the bow to preview its path.";
        return;
      }
      double f = p.getTicksUsingItem() / 20.0;
      velocity = Math.min(1, (f * f + 2 * f) / 3) * 3;
      gravity = .05;
    } else if (stack.is(Items.ENDER_PEARL) || stack.is(Items.SNOWBALL) || stack.is(Items.EGG)) {
      velocity = 1.5;
      gravity = .03;
    } else {
      trajectoryStatus = "Bow, pearl, snowball and egg supported.";
      return;
    }
    Vec3 pos = p.getEyePosition().add(0, -.1, 0),
        movement =
            Vec3.directionFromRotation(p.getXRot(), p.getYRot())
                .scale(velocity)
                .add(p.getDeltaMovement());
    trajectoryStatus = "Estimated path; spread and moving entities unknown.";
    for (int i = 0; i < 160; i++) {
      Vec3 next = pos.add(movement);
      BlockPos block = BlockPos.containing(next);
      if (mc.level
              .getChunkSource()
              .getChunk(block.getX() >> 4, block.getZ() >> 4, ChunkStatus.FULL, false)
          == null) {
        trajectoryStatus = "Path ends at unknown terrain.";
        break;
      }
      var hit =
          mc.level.clip(
              new ClipContext(pos, next, ClipContext.Block.COLLIDER, ClipContext.Fluid.NONE, p));
      if (hit.getType() != HitResult.Type.MISS) {
        line(out, pos, hit.getLocation(), 0xffe9bf78, false);
        box(
            out,
            new AABB(hit.getLocation().subtract(.1, .1, .1), hit.getLocation().add(.1, .1, .1)),
            0xffefcb8b,
            false);
        break;
      }
      line(out, pos, next, 0xffd7b36b, false);
      pos = next;
      movement =
          movement.scale(mc.level.getFluidState(block).isEmpty() ? .99 : .6).add(0, -gravity, 0);
    }
  }

  public static void render() {
    var mc = Minecraft.getInstance();
    if (mc.level == null) return;
    try (var ignored = mc.levelRenderer.collectPerFrameRenderThreadGizmos()) {
      renderedGeometry = snapshot.size();
      for (Draw draw : snapshot) {
        GizmoProperties properties =
            draw.box
                ? Gizmos.cuboid(new AABB(draw.a, draw.b), GizmoStyle.stroke(draw.color, 1.5f))
                : Gizmos.line(draw.a, draw.b, draw.color, 1.4f);
        if (draw.through) properties.setAlwaysOnTop();
      }
    }
  }
}
