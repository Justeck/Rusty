package dev.justeck.rusty.core;

import java.util.Objects;

/**
 * Read-only publication gate for ordinary player rotation; it never constructs or sends a packet.
 */
public final class SchematicLook {
  public record Context(Object player, Object connection, Object world, String scope) {
    public Context {
      Objects.requireNonNull(player);
      Objects.requireNonNull(connection);
      Objects.requireNonNull(world);
      Objects.requireNonNull(scope);
    }
  }

  public record Pose(double x, double y, double z) {
    public Pose {
      if (!Double.isFinite(x) || !Double.isFinite(y) || !Double.isFinite(z))
        throw new IllegalArgumentException("Nonfinite pose");
    }
  }

  public enum Gate {
    READY,
    WAITING,
    STALE,
    TIMED_OUT,
    CANCELLED
  }

  private Context publishedContext;
  private float publishedYaw, publishedPitch;

  private static boolean angles(float yaw, float pitch) {
    return Float.isFinite(yaw) && Float.isFinite(pitch) && pitch >= -90 && pitch <= 90;
  }

  private static float wrap(float yaw) {
    float v = yaw % 360;
    if (v >= 180) v -= 360;
    if (v < -180) v += 360;
    return v == 0 ? 0 : v;
  }

  private static boolean sameContext(Context a, Context b) {
    return a != null
        && b != null
        && a.player == b.player
        && a.connection == b.connection
        && a.world == b.world
        && a.scope.equals(b.scope);
  }

  public static final class Aim {
    public final Context context;
    public final Pose pose;
    public final float yaw, pitch;
    public final int started;
    private volatile boolean cancelled;

    private Aim(Context c, Pose p, float y, float x, int t) {
      context = c;
      pose = p;
      yaw = wrap(y);
      pitch = x == 0 ? 0 : x;
      started = t;
    }

    public void cancel() {
      cancelled = true;
    }

    public boolean same(Context c, Pose p, float y, float x) {
      return !cancelled
          && angles(y, x)
          && sameContext(context, c)
          && pose.equals(p)
          && yaw == wrap(y)
          && pitch == x;
    }
  }

  public synchronized void superseded(Context context, float yaw, float pitch) {
    if (!angles(yaw, pitch)
        || !sameContext(publishedContext, context)
        || publishedYaw != wrap(yaw)
        || publishedPitch != pitch) clear();
  }

  public synchronized void clear() {
    publishedContext = null;
  }

  public synchronized void observe(Context c, float yaw, float pitch) {
    if (c == null || !angles(yaw, pitch)) {
      clear();
      return;
    }
    publishedContext = c;
    publishedYaw = wrap(yaw);
    publishedPitch = pitch == 0 ? 0 : pitch;
  }

  public Aim aim(Context c, Pose p, float yaw, float pitch, int tick) {
    Objects.requireNonNull(c);
    Objects.requireNonNull(p);
    if (!angles(yaw, pitch)) throw new IllegalArgumentException("Invalid look angles");
    return new Aim(c, p, yaw, pitch, tick);
  }

  public synchronized Gate check(Aim aim, Context current, Pose pose, int tick) {
    if (aim.cancelled) return Gate.CANCELLED;
    if (!sameContext(aim.context, current) || !aim.pose.equals(pose)) return Gate.STALE;
    if ((long) tick - aim.started > 100) return Gate.TIMED_OUT;
    return sameContext(publishedContext, current)
            && aim.yaw == publishedYaw
            && aim.pitch == publishedPitch
        ? Gate.READY
        : Gate.WAITING;
  }
}
