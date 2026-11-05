package dev.justeck.rusty;

import java.util.function.Consumer;
import net.minecraft.client.model.geom.builders.UVPair;
import net.minecraft.client.resources.model.geometry.BakedQuad;
import org.joml.Vector3f;

/** Preserve original material/tint and interpolate its original UVs along four frame strips. */
public final class TextureFrames {
  public static final java.util.concurrent.atomic.AtomicLong framedQuads =
      new java.util.concurrent.atomic.AtomicLong();

  private TextureFrames() {}

  public static void emit(BakedQuad q, double edge, Consumer<BakedQuad> output) {
    if (edge >= .5) {
      output.accept(q);
      return;
    }
    framedQuads.incrementAndGet();
    float e = (float) edge;
    output.accept(crop(q, 0, 0, 1, e));
    output.accept(crop(q, 0, 1 - e, 1, 1));
    output.accept(crop(q, 0, e, e, 1 - e));
    output.accept(crop(q, 1 - e, e, 1, 1 - e));
  }

  private static BakedQuad crop(BakedQuad q, float s0, float t0, float s1, float t1) {
    return new BakedQuad(
        point(q, s0, t0),
        point(q, s1, t0),
        point(q, s1, t1),
        point(q, s0, t1),
        uv(q, s0, t0),
        uv(q, s1, t0),
        uv(q, s1, t1),
        uv(q, s0, t1),
        q.direction(),
        q.materialInfo());
  }

  private static Vector3f point(BakedQuad q, float s, float t) {
    return new Vector3f(q.position0())
        .mul((1 - s) * (1 - t))
        .fma(s * (1 - t), q.position1())
        .fma(s * t, q.position2())
        .fma((1 - s) * t, q.position3());
  }

  private static long uv(BakedQuad q, float s, float t) {
    float[] w = {(1 - s) * (1 - t), s * (1 - t), s * t, (1 - s) * t};
    float u = 0, v = 0;
    for (int i = 0; i < 4; i++) {
      u += UVPair.unpackU(q.packedUV(i)) * w[i];
      v += UVPair.unpackV(q.packedUV(i)) * w[i];
    }
    return UVPair.pack(u, v);
  }
}
