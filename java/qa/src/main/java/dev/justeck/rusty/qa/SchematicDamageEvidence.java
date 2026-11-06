package dev.justeck.rusty.qa;

import dev.justeck.rusty.RustySchematics;
import dev.justeck.rusty.SchematicConstruction;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientPacketListener;
import net.minecraft.server.level.ServerPlayer;

/** Owned-fixture observations only. Never synthesizes a packet, health receipt or placement ack. */
public final class SchematicDamageEvidence {
  private record Snapshot(
      float health,
      float absorption,
      int hurtTime,
      int cooldown,
      boolean invulnerable,
      boolean abilityInvulnerable,
      boolean sourceInvulnerable,
      List<String> effects) {
    static Snapshot of(ServerPlayer player) {
      return new Snapshot(
          player.getHealth(),
          player.getAbsorptionAmount(),
          player.hurtTime,
          player.damageCooldownTime,
          player.isInvulnerable(),
          player.getAbilities().invulnerable,
          player.isInvulnerableTo(player.level(), player.damageSources().generic()),
          player.getActiveEffects().stream().map(Object::toString).toList());
    }
  }

  private record ServerEvidence(
      Snapshot before, Snapshot after, boolean success, int result, String error) {}

  private static volatile ServerEvidence serverEvidence;
  private static final List<Map<String, Object>> history = new ArrayList<>();
  private static boolean armed, absorbed, activeAtReceipt;
  private static Object listener, player, level;
  private static float initialHealth, initialAbsorption, minimumReceivedHealth;
  private static int initialUses, initialClock, damageReceipts, healthReceipts;
  private static Map<net.minecraft.core.BlockPos, net.minecraft.world.level.block.state.BlockState>
      initialCells = Map.of();

  private static void require(boolean condition, String message) {
    if (!condition) throw new AssertionError(message + "; damage evidence=" + current());
  }

  public static void begin(boolean absorptionCase) {
    var mc = Minecraft.getInstance();
    require(SchematicConstruction.active(), "Damage baseline requires an active construction run");
    armed = true;
    absorbed = absorptionCase;
    listener = mc.getConnection();
    player = mc.player;
    level = mc.level;
    initialHealth = minimumReceivedHealth = mc.player.getHealth();
    initialAbsorption = mc.player.getAbsorptionAmount();
    initialUses = SchematicLookEvidence.uses;
    initialClock = ((Number) SchematicConstruction.status().get("clock")).intValue();
    damageReceipts = healthReceipts = 0;
    activeAtReceipt = false;
    serverEvidence = null;
    var cells =
        new LinkedHashMap<
            net.minecraft.core.BlockPos, net.minecraft.world.level.block.state.BlockState>();
    for (var at : RustySchematics.targets().keySet()) {
      require(SchematicConstruction.received(at), "Damage baseline requires settled target cells");
      cells.put(at, mc.level.getBlockState(at));
    }
    initialCells = Map.copyOf(cells);
    require(
        initialHealth == 20 && (absorbed ? initialAbsorption >= 4 : initialAbsorption == 0),
        "Damage baseline must distinguish health loss from absorption loss");
    require(SchematicConstruction.placed == 0, "Damage baseline must precede placement");
  }

  public static boolean activeBaselineEstablished() {
    unchanged();
    require(SchematicConstruction.active(), "Construction stopped before damage was dispatched");
    return ((Number) SchematicConstruction.status().get("clock")).intValue() >= initialClock + 2;
  }

  public static void apply() {
    require(activeBaselineEstablished(), "Damage must follow actual active construction ticks");
    var mc = Minecraft.getInstance();
    var server = mc.getSingleplayerServer();
    var uuid = mc.player.getUUID();
    require(
        server != null, "Damage fixture requires the already-validated owned integrated server");
    server.execute(() -> {
      Snapshot before = null;
      try {
        var players = server.getPlayerList().getPlayers();
        if (players.size() != 1 || !players.getFirst().getUUID().equals(uuid))
          throw new IllegalStateException("Damage selector is not the sole owned QA player");
        var ownedPlayer = players.getFirst();
        before = Snapshot.of(ownedPlayer);
        if (before.sourceInvulnerable || before.cooldown > 10 || before.hurtTime > 0)
          throw new IllegalStateException("Owned damage baseline is invulnerable or cooling down");
        boolean[] success = {false};
        int[] result = {0};
        // Ordinary server command and callback. Do not suppress failures; also prove actual loss.
        server
            .getCommands()
            .performPrefixedCommand(
                server
                    .createCommandSourceStack()
                    .withCallback((ok, value) -> {
                      success[0] = ok;
                      result[0] = value;
                    }),
                SchematicAcceptance.damageCommand());
        serverEvidence =
            new ServerEvidence(before, Snapshot.of(ownedPlayer), success[0], result[0], null);
      } catch (Throwable error) {
        serverEvidence = new ServerEvidence(before, null, false, 0, error.toString());
      }
    });
  }

  /** Called after vanilla thread dispatch, immediately before the real entity damage handler. */
  public static void receivedDamage(ClientPacketListener connection, int entityId) {
    var mc = Minecraft.getInstance();
    if (!armed
        || connection != listener
        || mc.player != player
        || mc.level != level
        || mc.player.getId() != entityId) return;
    damageReceipts++;
    activeAtReceipt |= SchematicConstruction.active();
  }

  /** Called after the ordinary incoming health packet has changed the local player. */
  public static void receivedHealth(ClientPacketListener connection) {
    var mc = Minecraft.getInstance();
    if (!armed || connection != listener || mc.player != player || mc.level != level) return;
    healthReceipts++;
    minimumReceivedHealth = Math.min(minimumReceivedHealth, mc.player.getHealth());
  }

  public static boolean stoppedWithProof() {
    unchanged();
    var evidence = serverEvidence;
    if (evidence == null) return false;
    require(
        evidence.error == null && evidence.success && evidence.result == 1,
        "Ordinary server damage command must succeed");
    require(
        evidence.before.health == initialHealth && evidence.before.absorption == initialAbsorption,
        "Server and client damage baselines agree");
    if (absorbed) {
      require(
          evidence.after.health == evidence.before.health
              && evidence.after.absorption < evidence.before.absorption,
          "Real server damage must consume absorption while health remains unchanged");
    } else {
      require(
          evidence.after.health < evidence.before.health && evidence.after.absorption == 0,
          "Real server damage must reduce health");
    }
    if (damageReceipts == 0 || (!absorbed && healthReceipts == 0)) return false;
    require(activeAtReceipt, "The real damage receipt must arrive during active construction");
    require(
        !SchematicConstruction.active() && SchematicConstruction.result.startsWith("Damage"),
        "Received damage stops construction");
    if (!absorbed)
      require(minimumReceivedHealth < initialHealth, "Client received actual health loss");
    return true;
  }

  public static void unchanged() {
    require(
        SchematicLookEvidence.uses == initialUses && SchematicConstruction.placed == 0,
        "Damage case must dispatch zero new item uses and count zero placements");
    var mc = Minecraft.getInstance();
    for (var cell : initialCells.entrySet())
      require(
          SchematicConstruction.received(cell.getKey())
              && mc.level.getBlockState(cell.getKey()).equals(cell.getValue()),
          "Damage target must remain its original settled received state");
  }

  public static boolean finishNoResume() {
    require(stoppedWithProof(), "Damage proof must remain valid after the no-resume window");
    history.add(current());
    armed = false;
    return true;
  }

  private static Map<String, Object> current() {
    var result = new LinkedHashMap<String, Object>();
    result.put("absorbed", absorbed);
    result.put("initialHealth", initialHealth);
    result.put("initialAbsorption", initialAbsorption);
    result.put("initialClock", initialClock);
    result.put("initialUses", initialUses);
    result.put("damageReceipts", damageReceipts);
    result.put("activeAtReceipt", activeAtReceipt);
    result.put("healthReceipts", healthReceipts);
    result.put("minimumReceivedHealth", minimumReceivedHealth);
    result.put("server", serverEvidence);
    return result;
  }

  public static Map<String, Object> receipt() {
    var result = new LinkedHashMap<String, Object>();
    result.put("completed", List.copyOf(history));
    result.put("current", current());
    return result;
  }
}
