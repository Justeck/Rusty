package dev.justeck.rusty;

import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.phys.*;

public final class RustyPlanner {
  private static BlockPos anchor;
  private static String scope = "";
  private static boolean place;

  public record Cell(BlockPos pos, int color) {}

  public static List<Cell> cells = List.of();

  public static void anchor() {
    var mc = Minecraft.getInstance();
    if (mc.hitResult instanceof BlockHitResult hit) {
      anchor = hit.getBlockPos().relative(hit.getDirection());
      scope = RustyClient.scope();
      RustyClient.notice = "Plan anchored.";
    }
  }

  public static void placeNext() {
    place = true;
  }

  public static void tick(boolean focused) {
    var mc = Minecraft.getInstance();
    if (anchor == null
        || !scope.equals(RustyClient.scope())
        || !RustyClient.active("build_planner")) {
      anchor = null;
      place = false;
      cells = List.of();
      return;
    }
    var cfg = RustyClient.config;
    var list = new ArrayList<Cell>();
    BlockItem held = mc.player.getMainHandItem().getItem() instanceof BlockItem b ? b : null;
    for (int x = 0; x < cfg.plannerWidth; x++)
      for (int y = 0; y < cfg.plannerHeight; y++)
        for (int z = 0; z < cfg.plannerDepth; z++) {
          boolean wall = x == 0 || z == 0 || x == cfg.plannerWidth - 1 || z == cfg.plannerDepth - 1;
          if (cfg.plannerHollow
              && !wall
              && !(cfg.plannerFloor && y == 0)
              && !(cfg.plannerRoof && y == cfg.plannerHeight - 1)) continue;
          BlockPos pos = anchor.offset(x, y, z);
          boolean loaded =
              mc.level
                      .getChunkSource()
                      .getChunk(pos.getX() >> 4, pos.getZ() >> 4, ChunkStatus.FULL, false)
                  != null;
          int color =
              !loaded
                  ? 0xff777777
                  : held != null && mc.level.getBlockState(pos).is(held.getBlock())
                      ? 0xff68b879
                      : mc.level.getBlockState(pos).canBeReplaced() ? 0xffd6a058 : 0xffcc6262;
          list.add(new Cell(pos, color));
        }
    list.sort(Comparator.comparingDouble(c -> c.pos.distToCenterSqr(mc.player.position())));
    cells = List.copyOf(list.stream().limit(128).toList());
    if (!place) return;
    place = false;
    if (!focused
        || held == null
        || mc.gameMode == null
        || !RustyClient.POLICY.allows("build_planner")
        || mc.player.isUsingItem()) return;
    for (Cell cell : list) {
      if (cell.color != 0xffd6a058 || cell.pos.distToCenterSqr(mc.player.getEyePosition()) > 20.25)
        continue;
      for (Direction face : Direction.values()) {
        BlockPos support = cell.pos.relative(face.getOpposite());
        var state = mc.level.getBlockState(support);
        if (state.isAir() || state.getCollisionShape(mc.level, support).isEmpty()) continue;
        Vec3 point =
            Vec3.atCenterOf(support)
                .add(face.getStepX() * .5, face.getStepY() * .5, face.getStepZ() * .5);
        var ray =
            mc.level.clip(
                new ClipContext(
                    mc.player.getEyePosition(),
                    point,
                    ClipContext.Block.COLLIDER,
                    ClipContext.Fluid.NONE,
                    mc.player));
        if (ray.getType() != HitResult.Type.BLOCK || !ray.getBlockPos().equals(support)) continue;
        Vec3 d = point.subtract(mc.player.getEyePosition());
        mc.player.setYRot((float) Math.toDegrees(Math.atan2(-d.x, d.z)));
        mc.player.setXRot((float) -Math.toDegrees(Math.atan2(d.y, Math.hypot(d.x, d.z))));
        mc.gameMode.useItemOn(
            mc.player, InteractionHand.MAIN_HAND, new BlockHitResult(point, face, support, false));
        RustyClient.notice = "Requested one ordinary placement; server decides the result.";
        return;
      }
    }
    RustyClient.notice = "No reachable empty plan cell with visible support.";
  }
}
