package dev.justeck.rusty.fabric;

import dev.justeck.rusty.RustyClient;
import dev.justeck.rusty.TextureFrames;
import java.util.*;
import java.util.function.Predicate;
import net.fabricmc.fabric.api.client.renderer.v1.Renderer;
import net.fabricmc.fabric.api.client.renderer.v1.mesh.*;
import net.fabricmc.fabric.api.client.renderer.v1.model.FabricBlockStateModel;
import net.minecraft.client.renderer.block.BlockAndTintGetter;
import net.minecraft.client.renderer.block.dispatch.*;
import net.minecraft.client.resources.model.sprite.Material;
import net.minecraft.core.*;
import net.minecraft.util.RandomSource;
import net.minecraft.world.level.block.state.BlockState;

/** Public Fabric model/mesh API adapter; no Indigo implementation dependency. */
final class FabricFrameModel implements BlockStateModel, FabricBlockStateModel {
  private final BlockStateModel delegate;
  private static final ThreadLocal<ArrayDeque<Sink>> POOL =
      ThreadLocal.withInitial(ArrayDeque::new);

  FabricFrameModel(BlockStateModel delegate) {
    this.delegate = delegate;
  }

  @Override
  public void collectParts(RandomSource random, List<BlockStateModelPart> output) {
    delegate.collectParts(random, output);
  }

  @Override
  public Material.Baked particleMaterial() {
    return delegate.particleMaterial();
  }

  @Override
  public int materialFlags() {
    return delegate.materialFlags();
  }

  @Override
  public Object createGeometryKey(
      BlockAndTintGetter level, BlockPos pos, BlockState state, RandomSource random) {
    return RustyClient.view.xray()
        ? null
        : ((FabricBlockStateModel) delegate).createGeometryKey(level, pos, state, random);
  }

  @Override
  public void emitQuads(
      QuadEmitter output,
      BlockAndTintGetter level,
      BlockPos pos,
      BlockState state,
      RandomSource random,
      Predicate<Direction> cull) {
    var view = RustyClient.view;
    if (!view.xray()) {
      ((FabricBlockStateModel) delegate).emitQuads(output, level, pos, state, random, cull);
      return;
    }
    boolean selected = view.selected(state, pos);
    if (!selected && !view.context()) return;
    var pool = POOL.get();
    var sink = pool.pollFirst();
    if (sink == null) sink = new Sink();
    sink.output = output;
    sink.selected = selected;
    sink.edge = (float) view.edge();
    try {
      ((FabricBlockStateModel) delegate)
          .emitQuads(sink.emitter, level, pos, state, random, selected ? direction -> false : cull);
    } finally {
      sink.output = null;
      pool.addFirst(sink);
    }
  }

  private static final class Sink {
    final QuadEmitter emitter = Renderer.get().quadEmitter(this::accept);
    QuadEmitter output;
    boolean selected;
    float edge;

    void accept(MutableQuadView q) {
      if (selected) {
        output.copyFrom(q).cullFace(null).emissive(true).emit();
        return;
      }
      TextureFrames.framedQuads.incrementAndGet();
      piece(q, 0, 0, 1, edge);
      piece(q, 0, 1 - edge, 1, 1);
      piece(q, 0, edge, edge, 1 - edge);
      piece(q, 1 - edge, edge, 1, 1 - edge);
    }

    void piece(QuadView source, float s0, float t0, float s1, float t1) {
      output.copyFrom(source);
      point(source, 0, s0, t0);
      point(source, 1, s1, t0);
      point(source, 2, s1, t1);
      point(source, 3, s0, t1);
      output.emit();
    }

    void point(QuadView q, int vertex, float s, float t) {
      float[] w = {(1 - s) * (1 - t), s * (1 - t), s * t, (1 - s) * t};
      float x = 0, y = 0, z = 0, u = 0, v = 0;
      for (int i = 0; i < 4; i++) {
        x += q.x(i) * w[i];
        y += q.y(i) * w[i];
        z += q.z(i) * w[i];
        u += q.u(i) * w[i];
        v += q.v(i) * w[i];
      }
      output.pos(vertex, x, y, z).uv(vertex, u, v);
    }
  }
}
