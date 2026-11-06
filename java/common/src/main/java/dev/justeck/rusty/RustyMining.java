package dev.justeck.rusty;

import dev.justeck.rusty.core.Mining;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.block.FallingBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.phys.*;

/** Explicit, transient resource automation. Inputs still pass through vanilla controls. */
public final class RustyMining {
  private static final Set<String> TERRAIN =
      Set.of(
          "stone",
          "deepslate",
          "cobbled_deepslate",
          "netherrack",
          "tuff",
          "calcite",
          "granite",
          "diorite",
          "andesite",
          "dirt",
          "grass_block",
          "coarse_dirt",
          "rooted_dirt",
          "end_stone",
          "basalt",
          "blackstone");
  private static Run run;
  public static String result = "Not started";
  public static int count;

  private static final class Run {
    Object level, connection, player;
    String scope, settings;
    Mining.Grid grid;
    Mining.Pos stand, start;
    Mining.Search search;
    int scan,
        phase,
        clock,
        actionStarted,
        collectUntil,
        emptySince = -1,
        previousSlot,
        ownedSlot = -1;
    float health;
    boolean attack;
    Mining.Action action;
    ArrayDeque<Mining.Action> actions = new ArrayDeque<>();
    Map<BlockState, Integer> cache = new HashMap<>();
  }

  public static boolean active() {
    return run != null;
  }

  public static boolean walking() {
    return run != null && run.action != null && !run.action.mine();
  }

  public static boolean steppingUp() {
    return walking() && run.action.pos().y() > Minecraft.getInstance().player.getY() + .5;
  }

  public static boolean attacking() {
    return run != null && run.attack;
  }

  static Mining.Pos pos(BlockPos p) {
    return new Mining.Pos(p.getX(), p.getY(), p.getZ());
  }

  static BlockPos block(Mining.Pos p) {
    return new BlockPos(p.x(), p.y(), p.z());
  }

  static Mining.Pos playerPos() {
    var p = Minecraft.getInstance().player;
    return new Mining.Pos(
        (int) Math.floor(p.getX()), (int) Math.round(p.getY()), (int) Math.floor(p.getZ()));
  }

  static String signature() {
    var s = RustyClient.config;
    return s.mineResource
        + ":"
        + s.mineCustom
        + ":"
        + s.mineRadius
        + ":"
        + s.mineGoal
        + ":"
        + s.mineHealth
        + ":"
        + s.mineFood
        + ":"
        + s.mineDurability
        + ":"
        + s.mineTunnels
        + ":"
        + s.minePreview;
  }

  public static void stop(String why) {
    var old = run;
    if (old == null) return;
    run = null;
    result = why;
    RustyClient.notice = "Auto Mine: " + why + " (" + count + " blocks)";
    var mc = Minecraft.getInstance();
    if (old.attack && mc.gameMode != null && !mc.options.keyAttack.isDown())
      mc.gameMode.stopDestroyBlock();
    if (mc.player == old.player && old.ownedSlot == mc.player.getInventory().getSelectedSlot())
      mc.player.getInventory().setSelectedSlot(old.previousSlot);
  }

  public static void manual() {
    if (active()) stop("Manual input took over");
  }

  public static boolean start() {
    var mc = Minecraft.getInstance();
    if (active()) {
      stop("Stopped");
      return false;
    }
    if (mc.player == null
        || mc.level == null
        || mc.gui.screen() != null
        || !mc.isWindowActive()
        || !RustyClient.POLICY.allows("auto_mine")
        || RustyAssistance.eating()
        || RustyNavigation.walking()
        || RustyClient.active("freecam")) {
      RustyClient.notice = "Auto Mine needs an active world, free controls and server permission.";
      return false;
    }
    var r = new Run();
    r.level = mc.level;
    r.connection = mc.getConnection();
    r.player = mc.player;
    r.scope = RustyClient.scope();
    r.settings = signature();
    r.health = mc.player.getHealth();
    r.previousSlot = mc.player.getInventory().getSelectedSlot();
    count = 0;
    run = r;
    try {
      snapshot(r);
      if (!guard()) return false;
      result = "Scanning received terrain";
      return true;
    } catch (RuntimeException e) {
      stop("Terrain exceeds the bounded search area");
      return false;
    }
  }

  static boolean loaded(Mining.Pos p) {
    var level = Minecraft.getInstance().level;
    return level != null
        && p.y() >= level.getMinY()
        && p.y() < level.getMaxY()
        && level.getChunkSource().getChunk(p.x() >> 4, p.z() >> 4, ChunkStatus.FULL, false) != null;
  }

  private record Tool(int slot, int ticks) {}

  static Tool tool(BlockState state, BlockPos at) {
    var mc = Minecraft.getInstance();
    var p = mc.player;
    if (p == null || mc.level == null) return null;
    float hardness = state.getDestroySpeed(mc.level, at);
    if (!Float.isFinite(hardness) || hardness < 0) return null;
    Tool best = null;
    for (int i = 0; i < 9; i++) {
      var stack = p.getInventory().getItem(i);
      if (stack.isEmpty()
          || (stack.isDamageableItem()
              && (stack.getMaxDamage() - stack.getDamageValue()) * 100L
                  <= stack.getMaxDamage() * (long) RustyClient.config.mineDurability)
          || (state.requiresCorrectToolForDrops() && !stack.isCorrectToolForDrops(state))) continue;
      float speed = stack.getDestroySpeed(state);
      if (!Float.isFinite(speed) || speed <= 0) continue;
      int ticks = Math.max(1, (int) Math.ceil(hardness * 30 / speed));
      if (ticks > 400) continue;
      if (best == null || ticks < best.ticks) best = new Tool(i, ticks);
    }
    return best;
  }

  static boolean settled(Mining.Pos p) {
    var level = Minecraft.getInstance().level;
    if (level == null) return false;
    var handler =
        ((dev.justeck.rusty.mixin.ClientPredictionAccessor) (Object) level).rusty$predictions();
    return !((dev.justeck.rusty.mixin.BlockPredictionAccessor) (Object) handler)
        .rusty$pending()
        .containsKey(block(p).asLong());
  }

  static boolean rawAir(Mining.Pos p) {
    return loaded(p) && Minecraft.getInstance().level.getBlockState(block(p)).isAir();
  }

  static int classify(Mining.Pos p) {
    if (!loaded(p) || !settled(p)) return Mining.UNKNOWN;
    var mc = Minecraft.getInstance();
    var at = block(p);
    var state = mc.level.getBlockState(at);
    if (state.isAir()) return Mining.AIR;
    String id = BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString();
    if (!state.getFluidState().isEmpty()
        || state.getBlock() instanceof FallingBlock
        || List.of(
                "lava",
                "fire",
                "magma",
                "cactus",
                "campfire",
                "sweet_berry",
                "powder_snow",
                "cobweb",
                "portal",
                "wither_rose",
                "pointed_dripstone")
            .stream()
            .anyMatch(id::contains)
        || !state.isCollisionShapeFullBlock(mc.level, at)) return Mining.HAZARD;
    if (!state.hasBlockEntity()
        && (TERRAIN.contains(id.replace("minecraft:", ""))
            || Mining.matches(RustyClient.config, id))) {
      var tool = tool(state, at);
      if (tool != null)
        return Mining.matches(RustyClient.config, id)
            ? Mining.resource(tool.ticks)
            : Mining.terrain(tool.ticks);
    }
    return Mining.WALL;
  }

  static void snapshot(Run r) {
    var level = Minecraft.getInstance().level;
    r.start = playerPos();
    r.stand = r.start;
    int radius = RustyClient.config.mineRadius;
    r.grid =
        new Mining.Grid(
            new Mining.Pos(r.start.x() - radius, level.getMinY(), r.start.z() - radius),
            radius * 2 + 1,
            level.getMaxY() - level.getMinY(),
            radius * 2 + 1);
    r.scan = 0;
    r.phase = 0;
    r.cache.clear();
    r.search = null;
  }

  static Mining.Grid local(Mining.Pos a, Mining.Pos b) {
    var origin =
        new Mining.Pos(
            Math.min(a.x(), b.x()) - 2, Math.min(a.y(), b.y()) - 2, Math.min(a.z(), b.z()) - 2);
    var g =
        new Mining.Grid(
            origin,
            Math.abs(a.x() - b.x()) + 5,
            Math.abs(a.y() - b.y()) + 6,
            Math.abs(a.z() - b.z()) + 5);
    for (int i = 0; i < g.size(); i++) {
      var p = g.position(i);
      g.set(p, classify(p));
    }
    return g;
  }

  /** Also called immediately before vanilla handleKeybinds: no stale mining input. */
  public static boolean guard() {
    if (run == null) return false;
    var r = run;
    var mc = Minecraft.getInstance();
    var p = mc.player;
    String reason = null;
    if (p == null
        || mc.level != r.level
        || mc.getConnection() != r.connection
        || p != r.player
        || !RustyClient.scope().equals(r.scope)) reason = "World or player changed";
    else if (mc.gui.screen() != null || !mc.isWindowActive())
      reason = "Menu opened or window lost focus";
    else if (!RustyClient.POLICY.allows("auto_mine")
        || RustyClient.active("freecam")
        || p.isDeadOrDying()
        || p.isSpectator()
        || p.isPassenger()
        || p.isSleeping()
        || p.getAbilities().flying
        || p.isFallFlying()) reason = "Player or permission is unavailable";
    else if (p.getHealth() < r.health) reason = "Player took damage";
    else if (p.getHealth() < RustyClient.config.mineHealth
        || p.getFoodData().getFoodLevel() < RustyClient.config.mineFood)
      reason = "Health or food is below the safety limit";
    else if (p.getInventory().getFreeSlot() < 0) reason = "Inventory is full";
    else if (mc.options.keyAttack.isDown()
        || mc.options.keyUse.isDown()
        || mc.options.keyUp.isDown()
        || mc.options.keyDown.isDown()
        || mc.options.keyLeft.isDown()
        || mc.options.keyRight.isDown()
        || mc.options.keyJump.isDown()
        || mc.options.keyShift.isDown()
        || mc.options.keySprint.isDown()) reason = "Manual input took over";
    else if (p.isUsingItem()
        || r.ownedSlot >= 0 && p.getInventory().getSelectedSlot() != r.ownedSlot)
      reason = "Held item changed";
    else if (!signature().equals(r.settings)) reason = "Settings changed; start again";
    else if (r.clock > 36000) reason = "Session time limit reached";
    if (reason != null) {
      stop(reason);
      return false;
    }
    r.health = p.getHealth();
    if (r.action == null || r.action.mine()) {
      var standing = local(r.stand, r.stand);
      if (r.action != null && r.action.mine() && rawAir(r.action.pos()) && !settled(r.action.pos()))
        standing.set(r.action.pos(), Mining.WALL);
      if (!playerPos().equals(r.stand) || !p.onGround() || !standing.standable(r.stand)) {
        stop("Player moved or standing terrain became unsafe");
        return false;
      }
    }
    if (r.action != null) {
      var a = r.action;
      var g = local(r.stand, a.pos());
      if (a.mine()) {
        if (!rawAir(a.pos()) && !g.canMine(a.pos(), RustyClient.config.mineTunnels)) {
          stop("Mining face became unsafe");
          return false;
        }
      } else if (!g.canWalk(r.stand, a.pos())) {
        stop("Walking route became unsafe");
        return false;
      }
    }
    return true;
  }

  static BlockHitResult aim(Mining.Pos target) {
    var mc = Minecraft.getInstance();
    var p = mc.player;
    var eye = p.getEyePosition();
    var center = Vec3.atCenterOf(block(target));
    if (eye.distanceTo(center) > p.blockInteractionRange()) return null;
    var delta = center.subtract(eye);
    for (var entity : mc.level.getEntities(p, p.getBoundingBox().expandTowards(delta).inflate(1))) {
      if (entity.isPickable()
          && !entity.isSpectator()
          && entity.getBoundingBox().inflate(entity.getPickRadius()).clip(eye, center).isPresent())
        return null;
    }
    var hit =
        mc.level.clip(
            new ClipContext(eye, center, ClipContext.Block.OUTLINE, ClipContext.Fluid.NONE, p));
    if (hit.getType() != HitResult.Type.BLOCK || !hit.getBlockPos().equals(block(target)))
      return null;
    var d = center.subtract(eye);
    p.setYRot((float) Math.toDegrees(Math.atan2(-d.x, d.z)));
    p.setXRot((float) -Math.toDegrees(Math.atan2(d.y, Math.sqrt(d.x * d.x + d.z * d.z))));
    mc.hitResult = hit;
    return hit;
  }

  public static void preInput() {
    if (!guard()) return;
    var r = run;
    if (r.action != null && r.action.mine() && r.attack) {
      if (rawAir(r.action.pos())) {
        r.attack = false;
        Minecraft.getInstance().gameMode.stopDestroyBlock();
        return;
      }
      var mc = Minecraft.getInstance();
      var tool = tool(mc.level.getBlockState(block(r.action.pos())), block(r.action.pos()));
      if (tool == null || tool.slot != r.ownedSlot || aim(r.action.pos()) == null)
        stop("Mining reach, tool or line of sight changed");
    }
  }

  public static void tick(boolean focused) {
    if (!guard()) return;
    var r = run;
    var mc = Minecraft.getInstance();
    r.clock++;
    if (count >= RustyClient.config.mineGoal && r.action == null && r.actions.isEmpty()) {
      if (r.clock >= r.collectUntil) stop("Block goal reached");
      return;
    }
    if (r.action != null) {
      var a = r.action;
      if (a.mine()) {
        if (rawAir(a.pos())) {
          r.attack = false;
          mc.gameMode.stopDestroyBlock();
          if (!settled(a.pos())) {
            if (r.clock - r.actionStarted > 600) stop("Server confirmation timed out");
            return;
          }
          if (r.emptySince < 0) r.emptySince = r.clock;
          if (r.clock - r.emptySince < 20) return;
          if (a.resource()) count++;
        } else {
          if (r.emptySince >= 0) {
            stop("Server restored the mined block");
            return;
          }
          if (r.clock - r.actionStarted > 500) {
            stop("Mining timed out or was rejected");
            return;
          }
          return;
        }
      } else {
        var target = Vec3.atBottomCenterOf(block(a.pos()));
        if (mc.player.position().distanceToSqr(target) > .08 || !mc.player.onGround()) {
          if (r.clock - r.actionStarted > 80) {
            stop("Movement stalled");
            return;
          }
          var delta = target.subtract(mc.player.position());
          mc.player.setYRot((float) Math.toDegrees(Math.atan2(-delta.x, delta.z)));
          return;
        }
        r.stand = a.pos();
      }
      r.actions.removeFirst();
      r.action = null;
      r.emptySince = -1;
      if (a.mine() && a.resource()) {
        var to = local(r.stand, a.pos()).collectionStep(r.stand, a.pos());
        if (Mining.collectAfter(r.actions, to, count >= RustyClient.config.mineGoal))
          result = "Collecting settled resource drops";
        r.collectUntil = r.clock + 10;
      } else if (!a.mine() && count >= RustyClient.config.mineGoal) r.collectUntil = r.clock + 10;
      if (count >= RustyClient.config.mineGoal && r.actions.isEmpty()) return;
    }
    switch (r.phase) {
      case 0 -> {
        int end = Math.min(r.scan + 8192, r.grid.size());
        while (r.scan < end) {
          var p = r.grid.position(r.scan++);
          int cell;
          if (!loaded(p) || !settled(p)) cell = Mining.UNKNOWN;
          else {
            var state = mc.level.getBlockState(block(p));
            cell = r.cache.computeIfAbsent(state, key -> classify(p));
          }
          r.grid.set(p, cell);
        }
        if (r.scan == r.grid.size()) r.phase = 1;
      }
      case 1 -> {
        result = "Planning a safe route";
        if (r.grid.prepare(32768)) {
          try {
            r.search = new Mining.Search(r.grid, r.start, RustyClient.config.mineTunnels, 32768);
            r.grid = null;
            r.phase = 2;
          } catch (IllegalArgumentException e) {
            stop("Player is not standing in safe terrain");
          }
        }
      }
      case 2 -> {
        var search = r.search.advance(256);
        switch (search.state()) {
          case WORKING -> {}
          case LIMITED -> stop("Route search limit reached; no partial route was executed");
          case NO_PATH -> stop("No safely reachable resource in received terrain");
          case FOUND -> {
            r.actions.addAll(search.plan().actions());
            r.search = null;
            r.phase = 3;
            result = "Mining " + Mining.RESOURCES[RustyClient.config.mineResource];
          }
        }
      }
      default -> {
        var a = r.actions.peekFirst();
        if (a == null) {
          snapshot(r);
          return;
        }
        var g = local(r.stand, a.pos());
        if (a.mine()) {
          var p = playerPos();
          if (p.x() == a.pos().x() && p.z() == a.pos().z()
              || !g.canMine(a.pos(), RustyClient.config.mineTunnels)) {
            stop("Unsafe mining face");
            return;
          }
          var tool = tool(mc.level.getBlockState(block(a.pos())), block(a.pos()));
          if (tool == null || aim(a.pos()) == null) {
            stop("No suitable tool or reachable mining face");
            return;
          }
          mc.player.getInventory().setSelectedSlot(tool.slot);
          r.ownedSlot = tool.slot;
          r.attack = true;
        } else {
          if (!g.canWalk(r.stand, a.pos())) {
            stop("Unsafe walking step");
            return;
          }
          var delta = Vec3.atBottomCenterOf(block(a.pos())).subtract(mc.player.position());
          mc.player.setYRot((float) Math.toDegrees(Math.atan2(-delta.x, delta.z)));
        }
        r.action = a;
        r.actionStarted = r.clock;
        r.emptySince = -1;
      }
    }
  }

  public static Map<String, Object> status() {
    var m = new LinkedHashMap<String, Object>();
    m.put("active", active());
    m.put("count", count);
    m.put("result", result);
    if (run != null) {
      m.put("phase", run.phase);
      m.put("clock", run.clock);
      m.put("actions", run.actions.size());
      m.put("attack", run.attack);
    }
    return m;
  }

  public static List<Vec3[]> segments() {
    if (run == null || !RustyClient.config.minePreview) return List.of();
    var out = new ArrayList<Vec3[]>();
    var last = Vec3.atCenterOf(block(run.stand));
    for (var a : run.actions.stream().limit(256).toList()) {
      var next = Vec3.atCenterOf(block(a.pos()));
      out.add(new Vec3[] {last, next});
      last = next;
    }
    return out;
  }
}
