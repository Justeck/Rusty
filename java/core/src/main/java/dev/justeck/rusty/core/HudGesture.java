package dev.justeck.rusty.core;

/** Screen-space editing math, independent of game input and persisted settings. */
public final class HudGesture {
  private HudGesture() {}

  public record Result(double x, double y, double scale) {}

  public static Result move(double[] r, double dx, double dy, int width, int height) {
    return new Result(
        Math.clamp(r[0] + dx, 0, Math.max(0, width - r[2])),
        Math.clamp(r[1] + dy, 0, Math.max(0, height - r[3])),
        1);
  }

  public static Result resize(
      double[] r, double scale, int corner, double dx, double dy, int width, int height) {
    double bw = r[2] / scale, bh = r[3] / scale;
    double sx = corner % 2 == 0 ? -1 : 1, sy = corner < 2 ? -1 : 1;
    double next = Math.clamp(scale + (dx * sx * bw + dy * sy * bh) / (bw * bw + bh * bh), .5, 2);
    next = Math.min(next, Math.min(width / bw, height / bh));
    double x = r[0] + (corner % 2 == 0 ? r[2] - bw * next : 0),
        y = r[1] + (corner < 2 ? r[3] - bh * next : 0);
    return new Result(
        Math.clamp(x, 0, Math.max(0, width - bw * next)),
        Math.clamp(y, 0, Math.max(0, height - bh * next)),
        next);
  }
}
