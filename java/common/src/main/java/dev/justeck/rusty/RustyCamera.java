package dev.justeck.rusty;

import net.minecraft.client.Minecraft;
import net.minecraft.world.phys.Vec3;

public final class RustyCamera {
  public static Vec3 position = Vec3.ZERO;
  public static float yaw, pitch;
  private static boolean attached;

  public static void reset() {
    attached = false;
    position = Vec3.ZERO;
    yaw = 0;
    pitch = 0;
  }

  public static void tick() {
    var mc = Minecraft.getInstance();
    if (!RustyClient.active("freecam") || mc.player == null) {
      attached = false;
      return;
    }
    if (!attached) {
      position = mc.player.getEyePosition();
      yaw = mc.player.getYRot();
      pitch = mc.player.getXRot();
      attached = true;
    }
    if (mc.gui.screen() != null || !mc.isWindowActive()) return;
    double f = (mc.options.keyUp.isDown() ? 1 : 0) - (mc.options.keyDown.isDown() ? 1 : 0);
    double s = (mc.options.keyRight.isDown() ? 1 : 0) - (mc.options.keyLeft.isDown() ? 1 : 0);
    double up = (mc.options.keyJump.isDown() ? 1 : 0) - (mc.options.keyShift.isDown() ? 1 : 0);
    double rad = Math.toRadians(yaw), speed = RustyClient.config.cameraSpeed;
    Vec3 delta =
        new Vec3(-Math.sin(rad) * f + Math.cos(rad) * s, up, Math.cos(rad) * f + Math.sin(rad) * s);
    if (delta.lengthSqr() > 1) delta = delta.normalize();
    position = position.add(delta.scale(speed));
  }

  public static void turn(double dx, double dy) {
    yaw += (float) dx * .15f;
    pitch = Math.clamp(pitch + (float) dy * .15f, -90, 90);
  }
}
