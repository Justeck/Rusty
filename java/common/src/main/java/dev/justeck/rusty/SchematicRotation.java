package dev.justeck.rusty;

import dev.justeck.rusty.core.SchematicLook;
import net.minecraft.client.Minecraft;
import net.minecraft.network.Connection;
import net.minecraft.network.protocol.game.ServerboundMovePlayerPacket;

/** Observes ordinary vanilla playerSync rotation. Construction never sends a rotation packet. */
public final class SchematicRotation {
  private static final SchematicLook LOOK = new SchematicLook();

  public static void reset() {
    LOOK.clear();
  }

  static SchematicLook.Context context() {
    var mc = Minecraft.getInstance();
    return new SchematicLook.Context(mc.player, mc.getConnection(), mc.level, RustyClient.scope());
  }

  static SchematicLook.Pose pose() {
    var p = Minecraft.getInstance().player;
    return new SchematicLook.Pose(p.getX(), p.getY(), p.getZ());
  }

  public static void sent(Connection connection, ServerboundMovePlayerPacket packet) {
    var mc = Minecraft.getInstance();
    if (!packet.hasRotation()
        || mc.player == null
        || mc.level == null
        || mc.getConnection() == null
        || mc.getConnection().getConnection() != connection) return;
    LOOK.observe(context(), packet.getYRot(0), packet.getXRot(0));
  }

  public static void airUse(Connection connection, float yaw, float pitch) {
    var mc = Minecraft.getInstance();
    if (mc.player == null
        || mc.level == null
        || mc.getConnection() == null
        || mc.getConnection().getConnection() != connection) return;
    LOOK.superseded(context(), yaw, pitch);
  }

  static SchematicLook.Aim stage(SchematicPlacement.Probe probe, int started) {
    return LOOK.aim(context(), pose(), probe.yaw(), probe.pitch(), started);
  }

  static SchematicLook.Gate check(SchematicLook.Aim aim, int tick) {
    return LOOK.check(aim, context(), pose(), tick);
  }
}
