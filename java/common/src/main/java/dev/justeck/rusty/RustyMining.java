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
  private static Mining.Pos first, second;
  private static Object selectionLevel, selectionConnection;
  private static String selectionScope;

  public static boolean areaActive() {
    return run != null && run.area != null;
  }

  public static boolean resourceActive() {
    return active() && !areaActive();
  }

  private static int healthLimit() {
    return areaActive() ? RustyClient.config.clearHealth : RustyClient.config.mineHealth;
  }

  private static int foodLimit() {
    return areaActive() ? RustyClient.config.clearFood : RustyClient.config.mineFood;
  }

  private static int durabilityLimit() {
    return areaActive() ? RustyClient.config.clearDurability : RustyClient.config.mineDurability;
  }

  private static boolean tunnels() {
    return !areaActive() && RustyClient.config.mineTunnels;
  }

  private static boolean quota() {
    return !areaActive() && count >= RustyClient.config.mineGoal;
  }

  public static void clearSelection() {
    first = null;
    second = null;
    selectionLevel = null;
    selectionConnection = null;
    selectionScope = null;
  }

  private static Mining.Region region() {
    return first != null && second != null ? new Mining.Region(first, second) : null;
  }

  public static String selectionText() {
    var r = region();
    if (r != null)
      return (r.max().x() - r.min().x() + 1)
          + " × "
          + (r.max().y() - r.min().y() + 1)
          + " × "
          + (r.max().z() - r.min().z() + 1)
          + " — "
          + r.volume()
          + " blocks";
    return first != null
        ? "First corner set; choose the second"
        : second != null
            ? "Second corner set; choose the first"
            : "Aim at blocks and set both corners";
  }

  public static String selectionDetails() {
    return "First: " + first + "; second: " + second;
  }

  public static boolean selectCorner(boolean last) {
    var mc = Minecraft.getInstance();
    if (mc.level == null
        || mc.player == null
        || !(mc.hitResult instanceof BlockHitResult hit)
        || hit.getType() != HitResult.Type.BLOCK) {
      RustyClient.notice = "Aim at a block before opening the menu";
      return false;
    }
    if (selectionLevel != mc.level
        || selectionConnection != mc.getConnection()
        || !Objects.equals(selectionScope, RustyClient.scope())) clearSelection();
    var at = pos(hit.getBlockPos());
    if (!loaded(at) || !settled(at)) {
      RustyClient.notice = "Corner is not settled received terrain";
      return false;
    }
    var a = last ? first : at;
    var b = last ? at : second;
    try {
      var candidate = new Mining.Region(a != null ? a : at, b != null ? b : at);
      if (!insideBorder(candidate))
        throw new IllegalArgumentException("Selection crosses the current world border");
    } catch (IllegalArgumentException e) {
      RustyClient.notice = e.getMessage();
      return false;
    }
    first = a;
    second = b;
    selectionLevel = mc.level;
    selectionConnection = mc.getConnection();
    selectionScope = RustyClient.scope();
    RustyClient.notice = selectionText();
    return true;
  }

  private static final class Run {
    Mining.Region area;
    Mining.ClearSurvey survey;
    int surveyIndex, highest = Integer.MIN_VALUE, remaining, protectedCount;
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
    if (areaActive())
      return "area:"
          + s.clearHealth
          + ":"
          + s.clearFood
          + ":"
          + s.clearDurability
          + ":"
          + s.clearPreview;
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
    RustyClient.notice =
        (old.area != null ? "Area Clear: " : "Auto Mine: ") + why + " (" + count + " blocks)";
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
    return startMode(false);
  }

  public static boolean startArea() {
    return startMode(true);
  }

  private static boolean startMode(boolean clearing) {
    var mc = Minecraft.getInstance();
    if (active()) {
      if (areaActive() != clearing) {
        RustyClient.notice = "Stop the other mining mode first";
        return false;
      }
      stop("Stopped");
      return false;
    }
    if (mc.player == null
        || mc.level == null
        || mc.gui.screen() != null
        || !mc.isWindowActive()
        || !RustyClient.POLICY.allows(clearing ? "area_clear" : "auto_mine")
        || RustyAssistance.eating()
        || RustyNavigation.walking()
        || RustyClient.active("freecam")) {
      RustyClient.notice = "Auto Mine needs an active world, free controls and server permission.";
      return false;
    }
    var r = new Run();
    if (clearing) {
      if (selectionLevel != mc.level
          || selectionConnection != mc.getConnection()
          || !Objects.equals(selectionScope, RustyClient.scope())) clearSelection();
      r.area = region();
      if (r.area == null) {
        RustyClient.notice = "Set both corners in this world first";
        return false;
      }
      if (!insideBorder(r.area)) {
        RustyClient.notice = "Selection crosses the current world border";
        return false;
      }
      if (r.area.min().y() < mc.level.getMinY() || r.area.max().y() >= mc.level.getMaxY()) {
        RustyClient.notice = "Selection is outside this dimension's height";
        return false;
      }
    }
    r.level = mc.level;
    r.connection = mc.getConnection();
    r.player = mc.player;
    r.scope = RustyClient.scope();

    r.health = mc.player.getHealth();
    r.previousSlot = mc.player.getInventory().getSelectedSlot();
    count = 0;
    run = r;
    r.settings = signature();
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

  private static boolean insideBorder(Mining.Region area) {
    var border = Minecraft.getInstance().level.getWorldBorder();
    return area.withinBorder(
        border.getMinX(), border.getMinZ(), border.getMaxX(), border.getMaxZ());
  }

  private static int positionCell(int cell, Mining.Pos p) {
    if (!areaActive()) return cell;
    var border = Minecraft.getInstance().level.getWorldBorder();
    if (p.x() < border.getMinX()
        || p.z() < border.getMinZ()
        || p.x() + 1.0 > border.getMaxX()
        || p.z() + 1.0 > border.getMaxZ()) return Mining.WALL;
    return run.area.constrain(cell, p, run.highest);
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
                  <= stack.getMaxDamage() * (long) durabilityLimit())
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
            .anyMatch(id::contains)) return Mining.HAZARD;
    boolean full = state.isCollisionShapeFullBlock(mc.level, at);
    if (areaActive()) {
      var tool = state.hasBlockEntity() ? null : tool(state, at);
      return tool == null ? Mining.passive(full) : Mining.resourceShape(tool.ticks, full);
    }
    if (!full) return Mining.HAZARD;
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
    r.stand = playerPos();
    r.start = r.stand;
    r.cache.clear();
    r.search = null;
    if (r.area != null) {
      r.surveyIndex = 0;
      r.survey = new Mining.ClearSurvey();
      r.phase = 4;
    } else allocate(r);
  }

  static void allocate(Run r) {
    var level = Minecraft.getInstance().level;
    r.start = playerPos();
    r.stand = r.start;
    if (r.area != null) {
      var a = r.area.min();
      var b = r.area.max();
      var lo =
          new Mining.Pos(
              Math.min(a.x(), r.start.x()) - 2,
              Math.min(a.y(), r.start.y()) - 2,
              Math.min(a.z(), r.start.z()) - 2);
      var hi =
          new Mining.Pos(
              Math.max(b.x(), r.start.x()) + 2,
              Math.max(b.y(), r.start.y() + 1) + 2,
              Math.max(b.z(), r.start.z()) + 2);
      r.grid = new Mining.Grid(lo, hi.x() - lo.x() + 1, hi.y() - lo.y() + 1, hi.z() - lo.z() + 1);
      r.scan = 0;
      r.phase = 0;
      r.cache.clear();
      r.search = null;
      return;
    }
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
      int cell = classify(p);
      g.set(p, positionCell(cell, p));
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
    else if (r.area != null && !insideBorder(r.area))
      reason = "World border changed; selection is no longer inside it";
    else if (!RustyClient.POLICY.allows(areaActive() ? "area_clear" : "auto_mine")
        || RustyClient.active("freecam")
        || p.isDeadOrDying()
        || p.isSpectator()
        || p.isPassenger()
        || p.isSleeping()
        || p.getAbilities().flying
        || p.isFallFlying()) reason = "Player or permission is unavailable";
    else if (p.getHealth() < r.health) reason = "Player took damage";
    else if (p.getHealth() < healthLimit() || p.getFoodData().getFoodLevel() < foodLimit())
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
        if (!rawAir(a.pos()) && !g.canMine(a.pos(), tunnels())) {
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
    var at = block(target);
    // Aim into actual outline geometry, including thin carpets and slabs.
    // Every candidate still uses ordinary reach, occlusion and entity checks.
    for (var box :
        mc.level.getBlockState(at).getShape(mc.level, at).toAabbs().stream().limit(16).toList()) {
      var center = box.getCenter().add(at.getX(), at.getY(), at.getZ());
      if (eye.distanceTo(center) > p.blockInteractionRange()) continue;
      var delta = center.subtract(eye);
      boolean blocked = false;
      for (var entity :
          mc.level.getEntities(p, p.getBoundingBox().expandTowards(delta).inflate(1))) {
        if (entity.isPickable()
            && !entity.isSpectator()
            && entity
                .getBoundingBox()
                .inflate(entity.getPickRadius())
                .clip(eye, center)
                .isPresent()) {
          blocked = true;
          break;
        }
      }
      if (blocked) continue;
      var hit =
          mc.level.clip(
              new ClipContext(eye, center, ClipContext.Block.OUTLINE, ClipContext.Fluid.NONE, p));
      if (hit.getType() != HitResult.Type.BLOCK || !hit.getBlockPos().equals(at)) continue;
      p.setYRot((float) Math.toDegrees(Math.atan2(-delta.x, delta.z)));
      p.setXRot((float) -Math.toDegrees(Math.atan2(delta.y, Math.hypot(delta.x, delta.z))));
      mc.hitResult = hit;
      return hit;
    }
    return null;
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
    if (quota() && r.action == null && r.actions.isEmpty()) {
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
        if (Mining.collectAfter(r.actions, to, quota()))
          result = "Collecting settled resource drops";
        r.collectUntil = r.clock + 10;
      } else if (!a.mine() && quota()) r.collectUntil = r.clock + 10;
      if (quota() && r.actions.isEmpty()) return;
    }
    switch (r.phase) {
      case 4 -> {
        int end = Math.min(r.surveyIndex + 2048, r.area.volume());
        while (r.surveyIndex < end) {
          var p = r.area.position(r.surveyIndex++);
          int cell;
          if (!loaded(p) || !settled(p)) cell = Mining.UNKNOWN;
          else {
            var state = mc.level.getBlockState(block(p));
            cell = r.cache.computeIfAbsent(state, key -> classify(p));
          }
          r.survey.observe(p, cell);
        }
        result = "Reviewing selection — " + r.surveyIndex + "/" + r.area.volume() + " cells";
        if (r.surveyIndex == r.area.volume()) {
          r.remaining = r.survey.remaining;
          r.protectedCount = r.survey.protectedCount;
          if (r.survey.unknown > 0) {
            stop(r.survey.unknown + " unknown or pending cells remain; stopped");
            return;
          }
          if (r.survey.dangerous > 0) {
            stop(
                "Selection contains "
                    + r.survey.dangerous
                    + " hazardous or unstable cells; clear hazards manually");
            return;
          }
          if (r.survey.complete()) {
            stop("Area cleared");
            return;
          }
          r.highest = r.survey.highest;
          if (r.highest == Integer.MIN_VALUE) {
            stop(r.remaining + " protected or unsuitable blocks remain");
            return;
          }
          try {
            allocate(r);
          } catch (IllegalArgumentException e) {
            stop("Selection and player exceed bounded route area");
          }
        }
      }
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
          r.grid.set(p, positionCell(cell, p));
        }
        if (r.scan == r.grid.size()) r.phase = 1;
      }
      case 1 -> {
        result = "Planning a safe route";
        if (r.grid.prepare(32768)) {
          try {
            r.search = new Mining.Search(r.grid, r.start, tunnels(), 32768);
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
          case NO_PATH ->
              stop(
                  r.area != null
                      ? r.remaining
                          + " blocks remain ("
                          + r.protectedCount
                          + " protected); no safe route to the top layer"
                      : "No safely reachable resource in received terrain");
          case FOUND -> {
            r.actions.addAll(search.plan().actions());
            r.search = null;
            r.phase = 3;
            result =
                r.area != null
                    ? "Clearing the selected top layer"
                    : "Mining " + Mining.RESOURCES[RustyClient.config.mineResource];
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
          if (r.area != null && !r.area.contains(a.pos())) {
            stop("Refusing excavation outside the selection");
            return;
          }
          var p = playerPos();
          if (p.x() == a.pos().x() && p.z() == a.pos().z() || !g.canMine(a.pos(), tunnels())) {
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
    m.put("area", areaActive());
    m.put("selection", selectionText());
    m.put("result", result);
    if (run != null) {
      m.put("remaining", run.remaining);
      m.put("protected", run.protectedCount);
      m.put("phase", run.phase);
      m.put("clock", run.clock);
      m.put("actions", run.actions.size());
      m.put("attack", run.attack);
    }
    return m;
  }

  public static List<Vec3[]> segments() {
    var out = new ArrayList<Vec3[]>();
    var area = region();
    if (area != null && RustyClient.config.clearPreview)
      for (var edge : area.edges())
        out.add(
            new Vec3[] {
              new Vec3(edge[0].x(), edge[0].y(), edge[0].z()),
              new Vec3(edge[1].x(), edge[1].y(), edge[1].z())
            });
    if (run == null
        || !(areaActive() ? RustyClient.config.clearPreview : RustyClient.config.minePreview))
      return out;
    var last = Vec3.atCenterOf(block(run.stand));
    for (var a : run.actions.stream().limit(256).toList()) {
      var next = Vec3.atCenterOf(block(a.pos()));
      out.add(new Vec3[] {last, next});
      last = next;
    }
    return out;
  }
}
