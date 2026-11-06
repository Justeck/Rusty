package dev.justeck.rusty;

import dev.justeck.rusty.core.*;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.inventory.ContainerInput;
import net.minecraft.world.item.*;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;

/**
 * Explicit ordinary construction: bounded walking, receipt-gated inventory, exact received
 * verification.
 */
public final class SchematicConstruction {
  public static long receivedRevision;
  private static Run run, terminal;
  private static String terminalReason;

  public record Outcome(
      Integer initiallyMatching,
      int newlyCompleted,
      int placementActions,
      int finallyMatching,
      int occupied,
      int remainingMissing,
      int manual,
      int inaccessible,
      int unknown,
      int importedDetails,
      int scanned,
      int total,
      boolean surveyComplete,
      boolean receiptPending) {}

  public static Outcome outcome;
  private static final String BUILD_PERMISSION =
      "Construction requires Survival or Creative build permission";

  public static boolean buildEligible() {
    var mc = Minecraft.getInstance();
    return mc.player != null
        && mc.gameMode != null
        && (mc.gameMode.getPlayerMode() == net.minecraft.world.level.GameType.SURVIVAL
            || mc.gameMode.getPlayerMode() == net.minecraft.world.level.GameType.CREATIVE)
        && mc.player.getAbilities().mayBuild;
  }

  public static void terrainColumn(Object level, int x, int z) {
    receivedRevision++;
    if (run != null && run.level == level) run.watch.column(x, z);
    if (terminal != null && terminal.level == level) terminal.watch.column(x, z);
  }

  public static void terrainAll(Object level) {
    receivedRevision++;
    if (run != null && run.level == level) run.watch.all();
    if (terminal != null && terminal.level == level) terminal.watch.all();
  }

  public static String result = "Not started";
  public static int placed, walked, sourced, probeWork, surveyWork, routeWork;

  private record Stack(ItemStack value) {
    Stack {
      value = value.copy();
    }

    public boolean equals(Object other) {
      return other instanceof Stack s && ItemStack.matches(value, s.value);
    }

    public int hashCode() {
      return 0;
    }
  }

  private static final class Run {
    Object level, connection, player;
    String scope, border;
    float health;
    int coupled;
    int clock,
        phase,
        index,
        matched,
        obstructed,
        manual,
        unknown,
        previousSlot,
        ownedSlot = -1,
        started,
        sequence,
        ack = -1,
        transferState;
    List<Map.Entry<BlockPos, BlockState>> cells;
    Map<BlockPos, BlockState> planned;
    List<Map.Entry<BlockPos, BlockState>> candidates = new ArrayList<>();
    Map.Entry<BlockPos, BlockState> target;
    BlockPos stand, step;
    ArrayDeque<BlockPos> route = new ArrayDeque<>();
    Map<BlockPos, Set<BlockPos>> attempted = new HashMap<>();
    long transferFence;
    int transferSource, transferDestination;
    SchematicSafety.Transfer<Stack> transfer;
    SchematicPlacement.Probe pending;
    SchematicLook.Aim aim;
    BlockPos aimTarget;
    ArrayDeque<BlockPos> open;
    Map<BlockPos, BlockPos> parent;
    Set<BlockPos> seen;
    int expansions;
    SchematicSurvey watch;
    SchematicSurvey.Pass pass = new SchematicSurvey.Pass();
    SchematicSurvey.Counts counts = new SchematicSurvey.Counts();
    Set<BlockPos> completed = new HashSet<>(),
        completionCandidates = new HashSet<>(),
        inaccessible = new HashSet<>();
    Integer initiallyMatching;
    int detailCount, surveyTicks, refreshTicks;
    boolean coherent;
  }

  public static boolean active() {
    return run != null;
  }

  public static boolean walking() {
    return run != null && run.step != null;
  }

  public static boolean steppingUp() {
    return walking()
        && ascentJump(run.stand.getY(), run.step.getY(), Minecraft.getInstance().player.getY());
  }

  static boolean ascentJump(int sourceY, int targetY, double playerY) {
    // Release while clearing the step, so landing/alignment cannot immediately bounce again.
    return targetY > sourceY && targetY > playerY + .5;
  }

  private static Mining.Pos pos(BlockPos p) {
    return new Mining.Pos(p.getX(), p.getY(), p.getZ());
  }

  public static boolean received(BlockPos p) {
    return RustyMining.loaded(pos(p)) && RustyMining.settled(pos(p));
  }

  private static String border() {
    var b = Minecraft.getInstance().level.getWorldBorder();
    return b.getMinX() + "/" + b.getMinZ() + "/" + b.getMaxX() + "/" + b.getMaxZ();
  }

  public static boolean permitted() {
    return RustyClient.POLICY.allows("build_planner") && RustyClient.POLICY.allows("auto_mine");
  }

  public static void manual() {
    if (active()) stop("Manual input took over");
  }

  public static void damageReceipt(Object listener, Object entity, Object level, String scope) {
    var r = run;
    if (r != null
        && r.connection == listener
        && r.player == entity
        && r.level == level
        && r.scope.equals(scope)) stop("Damage received from server");
  }

  public static void stop(String reason) {
    var old = run;
    run = null;
    if (old == null) return;
    if (old.aim != null) old.aim.cancel();
    terminal = old;
    terminalReason = reason;
    old.refreshTicks = 0;
    publish(
        old,
        reason,
        old.coherent && old.pending == null && old.pass.coherent(old.watch.revision()));
    old.pass = new SchematicSurvey.Pass();
    old.surveyTicks = 0;
    resetSurvey(old);
    var mc = Minecraft.getInstance();
    if (mc.player == old.player
        && old.ownedSlot >= 0
        && mc.player.getInventory().getSelectedSlot() == old.ownedSlot)
      mc.player.getInventory().setSelectedSlot(old.previousSlot);
  }

  public static boolean start() {
    var mc = Minecraft.getInstance();
    if (active()) {
      stop("Stopped explicitly");
      return false;
    }
    if (!RustySchematics.ready() && RustySchematics.loaded()) {
      RustyClient.notice = RustySchematics.result;
      return false;
    }
    if (!RustySchematics.ready()
        || !RustyClient.active("build_planner")
        || mc.player == null
        || mc.level == null
        || mc.gameMode == null
        || mc.gui.screen() != null
        || !mc.isWindowActive()
        || !permitted()
        || RustyMining.active()
        || RustyNavigation.walking()
        || RustyAssistance.eating()
        || RustyClient.active("freecam")) {
      RustyClient.notice =
          "Construction needs ready preview, free controls and current-server view + movement +"
              + " assistance";
      return false;
    }
    if (!buildEligible()) {
      result = RustyClient.notice = BUILD_PERMISSION;
      return false;
    }
    terminal = null;
    outcome = null;
    var r = new Run();
    r.level = mc.level;
    r.connection = mc.getConnection();
    r.player = mc.player;
    r.scope = RustyClient.scope();
    r.border = border();
    r.health = mc.player.getHealth();
    r.stand = mc.player.blockPosition();
    r.previousSlot = mc.player.getInventory().getSelectedSlot();
    r.planned = Map.copyOf(RustySchematics.targets());
    r.cells = new ArrayList<>(RustySchematics.targets().entrySet());
    r.watch =
        new SchematicSurvey(
            r.cells.stream()
                .map(
                    e -> new Schematic.Pos(e.getKey().getX(), e.getKey().getY(), e.getKey().getZ()))
                .toList());
    r.detailCount = RustySchematics.document.detailCount();
    run = r;
    placed = walked = sourced = 0;
    result = "Surveying settled received targets";
    return guard();
  }

  public static boolean guard() {
    var r = run;
    if (r == null) return false;
    var mc = Minecraft.getInstance();
    var p = mc.player;
    if (p == null
        || p != r.player
        || mc.level != r.level
        || mc.getConnection() != r.connection
        || !r.scope.equals(RustyClient.scope())) {
      stop("World/player/connection changed");
      return false;
    }
    if (!buildEligible()) {
      stop(BUILD_PERMISSION + "; permission was revoked");
      return false;
    }
    boolean manual =
        mc.options.keyAttack.isDown()
            || mc.options.keyUse.isDown()
            || mc.options.keyUp.isDown()
            || mc.options.keyDown.isDown()
            || mc.options.keyLeft.isDown()
            || mc.options.keyRight.isDown()
            || mc.options.keyJump.isDown()
            || mc.options.keyShift.isDown()
            || mc.options.keySprint.isDown();
    String reason =
        SchematicSafety.stopReason(
            manual,
            permitted() && RustyClient.active("build_planner"),
            mc.gui.screen() == null && mc.isWindowActive(),
            p.getHealth(),
            r.health,
            p.getFoodData().getFoodLevel(),
            r.clock,
            RustyClient.config.plannerHealth,
            RustyClient.config.plannerFood);
    if (reason == null
        && (RustyMining.active()
            || RustyNavigation.walking()
            || RustyAssistance.eating()
            || RustyClient.active("freecam")
            || p.isDeadOrDying()
            || p.isSpectator()
            || p.isPassenger()
            || p.isSleeping()
            || p.getAbilities().flying
            || p.isFallFlying()
            || p.isUsingItem()
            || p.containerMenu != p.inventoryMenu)) reason = "Player/controls unavailable";
    if (reason == null && !r.border.equals(border())) reason = "World border changed";
    if (reason == null && r.ownedSlot >= 0 && p.getInventory().getSelectedSlot() != r.ownedSlot)
      reason = "Held slot changed";
    if (reason == null
        && r.step == null
        && (!p.onGround() || !p.blockPosition().equals(r.stand) || !safeStand(r.stand)))
      reason = "Standing terrain/support changed";
    if (reason == null
        && r.step != null
        && (!edge(r.stand, r.step)
            || p.position().distanceToSqr(Vec3.atBottomCenterOf(r.stand)) > 6))
      reason = "Walking terrain became unsafe";
    if (reason != null) {
      stop(reason);
      return false;
    }
    r.health = p.getHealth();
    return true;
  }

  private static boolean safeStand(BlockPos p) {
    var mc = Minecraft.getInstance();
    return mc.level.getWorldBorder().isWithinBounds(p) && terrain(p, p).standable(pos(p));
  }

  private static boolean edge(BlockPos a, BlockPos b) {
    var mc = Minecraft.getInstance();
    return mc.level.getWorldBorder().isWithinBounds(a)
        && mc.level.getWorldBorder().isWithinBounds(b)
        && terrain(a, b).canWalk(pos(a), pos(b));
  }

  private static Mining.Grid terrain(BlockPos a, BlockPos b) {
    var min =
        new Mining.Pos(
            Math.min(a.getX(), b.getX()) - 2,
            Math.min(a.getY(), b.getY()) - 2,
            Math.min(a.getZ(), b.getZ()) - 2);
    var grid =
        new Mining.Grid(
            min,
            Math.abs(a.getX() - b.getX()) + 5,
            Math.abs(a.getY() - b.getY()) + 6,
            Math.abs(a.getZ() - b.getZ()) + 5);
    var mc = Minecraft.getInstance();
    for (int i = 0; i < grid.size(); i++) {
      var p = grid.position(i);
      var at = new BlockPos(p.x(), p.y(), p.z());
      int cell = Mining.UNKNOWN;
      if (RustyMining.loaded(p) && mc.level.getWorldBorder().isWithinBounds(at)) {
        if (!RustyMining.settled(p)) {
          if (run != null && run.pending != null && run.pending.writes().containsKey(at))
            cell = Mining.passive(false);
        } else {
          var state = mc.level.getBlockState(at);
          cell =
              state.isAir()
                  ? Mining.AIR
                  : RustyMining.hazardous(state)
                      ? Mining.HAZARD
                      : Mining.passive(state.isCollisionShapeFullBlock(mc.level, at));
        }
      }
      grid.set(p, cell);
    }
    return grid;
  }

  public static void inventoryReceipt(Object listener, int slot, int state, ItemStack stack) {
    var r = run;
    if (r != null && r.connection == listener && r.transfer != null)
      r.transfer.received(slot, new Stack(stack));
  }

  public static void placementAck(Object listener, int sequence) {
    if (run != null && run.connection == listener) run.ack = Math.max(run.ack, sequence);
    if (terminal != null && terminal.connection == listener)
      terminal.ack = Math.max(terminal.ack, sequence);
  }

  private static void survey(Run r) {
    r.pending = null;
    r.pass = new SchematicSurvey.Pass();
    r.surveyTicks = 0;
    resetSurvey(r);
  }

  private static void resetSurvey(Run r) {
    r.phase = 0;
    r.index = r.matched = r.obstructed = r.manual = r.unknown = r.coupled = 0;
    r.candidates.clear();
    r.target = null;
    r.counts = new SchematicSurvey.Counts();
    r.coherent = false;
  }

  private static ItemStack available(Item item) {
    var p = Minecraft.getInstance().player;
    for (int i = 0; i < 36; i++) {
      var stack = p.getInventory().getItem(i);
      if (stack.is(item) && !stack.isEmpty()) return stack;
    }
    return ItemStack.EMPTY;
  }

  private static boolean material(Run r, Item wanted) {
    if (!guard()) return false;
    var mc = Minecraft.getInstance();
    var inv = mc.player.getInventory();
    for (int i = 0; i < 9; i++)
      if (inv.getItem(i).is(wanted) && !inv.getItem(i).isEmpty()) {
        inv.setSelectedSlot(i);
        r.ownedSlot = i;
        return true;
      }
    int source = -1;
    for (int i = 9; i < 36; i++)
      if (inv.getItem(i).is(wanted) && !inv.getItem(i).isEmpty()) {
        source = i;
        break;
      }
    if (source < 0) {
      stop(
          "Missing material "
              + net.minecraft.core.registries.BuiltInRegistries.ITEM.getKey(wanted));
      return false;
    }
    int destination = r.previousSlot;
    var incoming = inv.getItem(source).copy();
    var displaced = inv.getItem(destination).copy();
    r.transferState = mc.player.inventoryMenu.getStateId();
    r.transfer =
        new SchematicSafety.Transfer<>(
            source, 36 + destination, new Stack(incoming), new Stack(displaced));
    r.started = r.clock;
    r.phase = 2;
    if (SchematicStatsBarrier.tainted()) {
      stop("Statistics response order unavailable; inventory was not moved");
      return false;
    }
    mc.gameMode.handleContainerInput(
        mc.player.inventoryMenu.containerId, source, destination, ContainerInput.SWAP, mc.player);
    r.transferSource = source;
    r.transferDestination = destination;
    r.transferFence = SchematicStatsBarrier.request();
    r.ownedSlot = destination;
    inv.setSelectedSlot(destination);
    result = "Waiting for ordinary server processing acknowledgment; displaced stack retained";
    return false;
  }

  public static void tick(boolean focused) {
    probeWork = surveyWork = routeWork = 0;
    SchematicPlacement.work = 0;
    if (run == null) {
      refreshTerminal();
      return;
    }
    if (!guard()) return;
    var r = run;
    var mc = Minecraft.getInstance();
    r.clock++;
    if (r.clock % 20 == 0) RustyClient.notice = "Construction: " + result;
    if (r.transfer != null) {
      if (r.transfer.rejected()) {
        stop("Inventory transfer rejected or changed");
        return;
      }
      if (SchematicStatsBarrier.tainted()) {
        stop("Statistics response order changed; stopped without placement");
        return;
      }
      if (!SchematicStatsBarrier.complete(r.transferFence)) {
        if (r.clock - r.started > 100) stop("Inventory processing acknowledgment timed out");
        return;
      }
      if (!r.transfer.matches(
          new Stack(mc.player.getInventory().getItem(r.transferSource)),
          new Stack(mc.player.getInventory().getItem(r.transferDestination)))) {
        stop("Server-corrected inventory does not match requested swap");
        return;
      }
      r.transfer = null;
      sourced++;
      r.phase = 1;
    }
    if (r.pending != null) {
      if (r.clock - r.started > 100) {
        stop("Placement acknowledgment stalled/rejected");
        return;
      }
      if (r.ack < r.sequence) return;
      for (var e : r.pending.writes().entrySet()) {
        if (!received(e.getKey())) return;
        if (!mc.level.getBlockState(e.getKey()).equals(e.getValue())) {
          stop("Server rejected/restored placement state");
          return;
        }
      }
      acceptPending(r);
      survey(r);
      return;
    }
    if (r.step != null) {
      if (mc.player.position().distanceToSqr(Vec3.atBottomCenterOf(r.step)) < .08
          && mc.player.onGround()) {
        r.stand = r.step;
        r.step = null;
        r.route.removeFirst();
        walked++;
        if (r.route.isEmpty()) {
          survey(r);
          return;
        }
      } else {
        if (r.clock - r.started > 80) {
          stop("Ordinary walking stalled");
          return;
        }
        var delta = Vec3.atBottomCenterOf(r.step).subtract(mc.player.position());
        mc.player.setYRot((float) Math.toDegrees(Math.atan2(-delta.x, delta.z)));
        return;
      }
    }
    if (!r.route.isEmpty()) {
      r.step = r.route.peekFirst();
      r.started = r.clock;
      return;
    }
    if (r.phase == 3) {
      route(r);
      return;
    }
    if (r.phase == 0) {
      if (!scan(r, true)) {
        stop("Received survey did not stabilize within its bound; unverified");
        return;
      }
      if (r.index < r.cells.size()) return;
      if (r.matched == r.cells.size()) {
        if (!r.coherent) return;
        stop("Complete: fresh received exact-state verification");
        return;
      }
      if (r.unknown > 0) {
        stop("Unknown/pending target cells require manual access");
        return;
      }
      if (r.candidates.isEmpty()) {
        stop("Remaining targets need manual work/access; no demolition");
        return;
      }
      r.index = 0;
      r.phase = 1;
      result = "Selecting supported low-layer placements";
    }
    if (r.phase == 1) {
      int count = 0;
      while (r.index < r.candidates.size() && count++ < 4 && SchematicPlacement.work < 96) {
        var e = r.candidates.get(r.index++);
        var req = SchematicMaterials.requirement(e.getValue());
        var stack = available(req.item());
        if (stack.isEmpty()) {
          stop(
              "Missing material "
                  + net.minecraft.core.registries.BuiltInRegistries.ITEM.getKey(req.item()));
          return;
        }
        var probe = SchematicPlacement.find(e.getKey(), e.getValue(), stack);
        probeWork = SchematicPlacement.work;
        if (probe == null) {
          r.attempted.computeIfAbsent(e.getKey(), k -> new HashSet<>()).add(r.stand);
          continue;
        }
        r.target = e;
        if (!material(r, req.item())) {
          r.index--;
          return;
        }
        // Re-probe the actual selected stack immediately before ordinary dispatch.
        SchematicPlacement.work = 0;
        probe = SchematicPlacement.find(e.getKey(), e.getValue(), mc.player.getMainHandItem());
        if (probe == null) {
          stop("Placement state/support changed");
          return;
        }
        mc.player.setYRot(probe.yaw());
        mc.player.setXRot(probe.pitch());
        // A new tail-tick look has not yet passed vanilla's earlier playerSync phase.
        // Keep aiming until an actual ordinary outgoing rotation matches this fresh probe.
        if (r.aim == null
            || !r.aim.same(
                SchematicRotation.context(),
                SchematicRotation.pose(),
                probe.yaw(),
                probe.pitch())) {
          int started = r.aim != null && e.getKey().equals(r.aimTarget) ? r.aim.started : r.clock;
          if (r.aim != null) r.aim.cancel();
          r.aim = SchematicRotation.stage(probe, started);
          r.aimTarget = e.getKey();
        }
        var look = SchematicRotation.check(r.aim, r.clock);
        if (look == SchematicLook.Gate.WAITING) {
          r.index--;
          result = "Waiting for ordinary player look synchronization";
          return;
        }
        if (look != SchematicLook.Gate.READY) {
          stop("Player look changed or synchronization stalled");
          return;
        }
        r.aim.cancel();
        r.aim = null;
        r.aimTarget = null;
        if (!guard()) return;
        mc.hitResult = probe.hit();
        r.pending = probe;
        r.started = r.clock;
        var handler =
            ((dev.justeck.rusty.mixin.ClientPredictionAccessor) (Object) mc.level)
                .rusty$predictions();
        r.completionCandidates.clear();
        for (var at : probe.writes().keySet())
          if (!mc.level.getBlockState(at).equals(RustySchematics.targets().get(at)))
            r.completionCandidates.add(at);
        r.coherent = false;
        mc.gameMode.useItemOn(mc.player, InteractionHand.MAIN_HAND, probe.hit());
        r.sequence = handler.currentSequence();
        result = "Waiting for received placement states";
        return;
      }
      if (r.index < r.candidates.size()) return;
      r.target = r.candidates.getFirst();
      beginRoute(r);
      return;
    }
  }

  private static boolean scan(Run r, boolean construction) {
    if (++r.surveyTicks > 1200) return false;
    if (r.pass.begin(r.watch.revision())) resetSurvey(r);
    if (r.pass.exhausted) return false;
    var mc = Minecraft.getInstance();
    int end = Math.min(r.cells.size(), r.index + 2048);
    while (r.index < end) {
      surveyWork++;
      var e = r.cells.get(r.index++);
      var at = e.getKey();
      var target = e.getValue();
      if (!received(at)) {
        r.counts.observe(0, false, false, false);
        r.unknown++;
        continue;
      }
      var actual = mc.level.getBlockState(at);
      if (actual.equals(target)) {
        r.counts.observe(1, false, false, r.completed.contains(at));
        r.matched++;
        continue;
      }
      if (!mc.level.getWorldBorder().isWithinBounds(at)
          || !SchematicPlacement.replaceable(at, target)) {
        r.counts.observe(3, false, false, false);
        r.obstructed++;
        continue;
      }
      var requirement = SchematicMaterials.requirement(target);
      r.counts.observe(2, !requirement.manual().isEmpty(), r.inaccessible.contains(at), false);
      if (!requirement.manual().isEmpty()) {
        r.manual++;
        continue;
      }
      if (requirement.units() == 0) {
        var secondary =
            new Schematic.Cell(
                new Schematic.Pos(at.getX(), at.getY(), at.getZ()),
                RustySchematics.describe(target));
        if (SchematicSafety.coupledCovered(
            secondary,
            p -> {
              var primary = r.planned.get(new BlockPos(p.x(), p.y(), p.z()));
              return primary == null ? null : RustySchematics.describe(primary);
            })) r.coupled++;
        else {
          r.manual++;
          r.counts.manual++;
        }
        continue;
      }
      if (!construction) continue;
      r.candidates.add(e);
      r.candidates.sort(
          Comparator.comparingInt((Map.Entry<BlockPos, BlockState> c) -> c.getKey().getY())
              .thenComparingDouble(c -> c.getKey().distToCenterSqr(mc.player.position())));
      if (r.candidates.size() > 64) r.candidates.removeLast();
    }
    if (r.index < r.cells.size()) {
      if (construction) result = "Surveying " + r.index + "/" + r.cells.size();
      return true;
    }
    r.coherent = r.pass.coherent(r.watch.revision());
    if (r.coherent && r.unknown == 0 && r.initiallyMatching == null && construction)
      r.initiallyMatching = r.matched;
    return true;
  }

  private static void acceptPending(Run r) {
    placed++;
    var mc = Minecraft.getInstance();
    for (var at : r.completionCandidates) {
      var wanted = r.planned.get(at);
      if (wanted != null && received(at) && mc.level.getBlockState(at).equals(wanted))
        r.completed.add(at);
    }
    r.completionCandidates.clear();
  }

  private static void publish(Run r, String reason, boolean valid) {
    var c = r.counts;
    int total = r.cells == null ? 0 : r.cells.size();
    outcome =
        new Outcome(
            r.initiallyMatching,
            r.completed.size(),
            placed,
            c.matching,
            c.occupied,
            c.missing,
            c.manual,
            c.inaccessible,
            c.unknown + total - c.scanned,
            r.detailCount,
            c.scanned,
            total,
            valid && c.scanned == total && c.unknown == 0,
            r.pending != null);
    result =
        reason
            + "; "
            + c.matching
            + "/"
            + total
            + " matching; "
            + r.completed.size()
            + " new cells / "
            + placed
            + " actions; "
            + (outcome.surveyComplete() ? "coherent settled survey" : "partial/unverified survey")
            + (r.pending != null ? "; dispatched receipt pending" : "");
    RustySchematics.result = result;
    RustyClient.notice = "Construction: " + result;
  }

  private static void refreshTerminal() {
    var r = terminal;
    if (r == null) return;
    var mc = Minecraft.getInstance();
    if (r.cells == null
        || mc.level != r.level
        || mc.player != r.player
        || mc.getConnection() != r.connection
        || !r.scope.equals(RustyClient.scope())) {
      publish(r, terminalReason, false);
      terminal = null;
      return;
    }
    // Read-only after stop, including a late receipt for the already dispatched ordinary action.
    if (r.pending != null && r.ack >= r.sequence) {
      boolean settled =
          r.pending.writes().keySet().stream().allMatch(SchematicConstruction::received);
      if (settled) {
        if (r.pending.writes().entrySet().stream()
            .allMatch(e -> mc.level.getBlockState(e.getKey()).equals(e.getValue())))
          acceptPending(r);
        r.pending = null;
        resetSurvey(r);
      }
    }
    boolean bounded = ++r.refreshTicks <= 1200 && scan(r, false);
    if (!bounded || r.index == r.cells.size()) {
      publish(r, terminalReason, bounded && r.coherent && r.pending == null);
      if (!bounded || (r.unknown == 0 && r.pending == null)) terminal = null;
      else resetSurvey(r);
    }
  }

  private static void beginRoute(Run r) {
    r.open = new ArrayDeque<>();
    r.parent = new HashMap<>();
    r.seen = new HashSet<>();
    r.open.add(r.stand);
    r.seen.add(r.stand);
    r.expansions = 0;
    r.phase = 3;
    result = "Searching bounded safe ordinary walking route";
  }

  private static void route(Run r) {
    int work = 0;
    BlockPos goal = r.target.getKey();
    while (!r.open.isEmpty() && work++ < 16 && r.expansions++ < 4096) {
      routeWork++;
      var at = r.open.removeFirst();
      if (!at.equals(r.stand)
          && !r.attempted.getOrDefault(goal, Set.of()).contains(at)
          && at.distManhattan(goal) <= 3
          && Math.abs(at.getY() - goal.getY()) <= 2) {
        var p = at;
        while (!p.equals(r.stand)) {
          r.route.addFirst(p);
          p = r.parent.get(p);
        }
        r.open = null;
        r.parent = null;
        r.seen = null;
        r.phase = 1;
        return;
      }
      for (int[] d : new int[][] {{1, 0}, {-1, 0}, {0, 1}, {0, -1}})
        for (int dy : new int[] {0, 1, -1}) {
          var next = at.offset(d[0], dy, d[1]);
          if (next.distManhattan(r.stand) > 64 || r.seen.contains(next) || !edge(at, next))
            continue;
          r.seen.add(next);
          r.parent.put(next, at);
          r.open.addLast(next);
          break;
        }
    }
    if (r.open.isEmpty() || r.expansions >= 4096) {
      r.inaccessible.add(goal);
      stop("Inaccessible orientation/support: bounded route exhausted; provide manual access");
    }
  }

  public static String details() {
    var o = outcome;
    String text =
        o == null
            ? "No retained construction outcome."
            : result.split(";", 2)[0]
                + "\n"
                + "Initially matching: "
                + (o.initiallyMatching() == null
                    ? "unknown (initial survey unfinished)"
                    : o.initiallyMatching())
                + "\nReceived newly completed cells: "
                + o.newlyCompleted()
                + "; ordinary placement actions: "
                + o.placementActions()
                + "\nFinal matching: "
                + o.finallyMatching()
                + "/"
                + o.total()
                + "; occupied: "
                + o.occupied()
                + "\nRemaining missing: "
                + o.remainingMissing()
                + " (manual: "
                + o.manual()
                + "; inaccessible: "
                + o.inaccessible()
                + ")"
                + "\nUnknown/unscanned: "
                + o.unknown()
                + "; scanned: "
                + o.scanned()
                + "/"
                + o.total()
                + "\nIgnored imported data details: "
                + o.importedDetails()
                + " (manual restoration)"
                + "\nSurvey: "
                + (o.surveyComplete() ? "coherent and settled" : "partial / unverified")
                + (o.receiptPending() ? "; dispatched receipt still pending" : "")
                + (terminal != null
                    ? "\nBounded read-only refresh in progress; construction stays stopped."
                    : "\nConstruction stays stopped; explicit Start is required.");
    if (SchematicFiles.commitMayFinish)
      text +=
          "\n"
              + "Export commit already began before cancellation; the chosen snapshot may finish"
              + " saving.";
    return text;
  }

  public static Map<String, Object> status() {
    var map = new LinkedHashMap<String, Object>();
    map.put("active", active());
    map.put("outcome", outcome);
    map.put("outcomeRefreshing", terminal != null);
    map.put("fileCommitMayFinish", SchematicFiles.commitMayFinish);
    map.put("placed", placed);
    map.put("walked", walked);
    map.put("sourced", sourced);
    map.put("result", result);
    map.put("probeWork", probeWork);
    map.put("surveyWork", surveyWork);
    map.put("routeWork", routeWork);
    if (run != null) map.put("surveyScanned", run.counts.scanned);
    if (run != null) {
      map.put("phase", run.phase);
      map.put("pendingCoupled", run.coupled);
      map.put("manual", run.manual);
      map.put("clock", run.clock);
      map.put("inventoryPending", run.transfer != null);
      map.put("placementPending", run.pending != null);
    }
    return map;
  }
}
