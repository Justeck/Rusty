package dev.justeck.rusty.qa;

import dev.justeck.rusty.SchematicConstruction;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.network.Connection;
import net.minecraft.network.protocol.Packet;
import net.minecraft.network.protocol.game.*;

/** Independent read-only trace of actual outgoing vanilla rotation and block-use packet order. */
public final class SchematicLookEvidence {
  private static Object player, world, connection;
  private static float yaw, pitch;
  private static long ordinal, rotation;
  public static int uses, violations;
  private static final List<Map<String, Object>> trace = new ArrayList<>();

  public static synchronized void resetUses() {
    uses = violations = 0;
    trace.clear();
  }

  public static synchronized List<Map<String, Object>> trace() {
    return List.copyOf(trace);
  }

  public static synchronized void observe(Connection sender, Packet<?> packet) {
    if (!"1".equals(System.getenv("RUSTY_QA_SCHEMATICS"))) return;
    var mc = Minecraft.getInstance();
    if (mc.player == null
        || mc.level == null
        || mc.getConnection() == null
        || mc.getConnection().getConnection() != sender) return;
    ordinal++;
    if (packet instanceof ServerboundMovePlayerPacket move && move.hasRotation()) {
      player = mc.player;
      world = mc.level;
      connection = sender;
      yaw = move.getYRot(0);
      pitch = move.getXRot(0);
      rotation = ordinal;
    }
    if (packet instanceof ServerboundUseItemOnPacket && SchematicConstruction.active()) {
      uses++;
      boolean matches =
          rotation > 0
              && rotation < ordinal
              && player == mc.player
              && world == mc.level
              && connection == sender
              && Float.isFinite(yaw)
              && Float.isFinite(pitch)
              && net.minecraft.util.Mth.wrapDegrees(yaw - mc.player.getYRot()) == 0
              && pitch == mc.player.getXRot();
      if (!matches) violations++;
      if (trace.size() < 64)
        trace.add(
            Map.of(
                "use",
                uses,
                "rotationOrdinal",
                rotation,
                "useOrdinal",
                ordinal,
                "yaw",
                mc.player.getYRot(),
                "publishedYaw",
                yaw,
                "pitch",
                mc.player.getXRot(),
                "publishedPitch",
                pitch,
                "matched",
                matches));
    }
  }
}
