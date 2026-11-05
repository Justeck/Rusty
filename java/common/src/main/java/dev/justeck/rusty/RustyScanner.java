package dev.justeck.rusty;

import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.core.SectionPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.level.chunk.status.ChunkStatus;

/** Palette-rejected sections cost no 4096-cell walk; all reads use already loaded chunks. */
final class RustyScanner {
  private static final ArrayDeque<SectionPos> queue = new ArrayDeque<>();
  private static final LinkedHashMap<Long, String> cache = new LinkedHashMap<>();
  private static int age;
  private static boolean partial;

  static void clear() {
    queue.clear();
    cache.clear();
    age = 0;
    partial = false;
  }

  static void tick() {
    var mc = Minecraft.getInstance();
    var level = mc.level;
    var cfg = RustyClient.config;
    if (level == null || mc.player == null) return;
    var origin = mc.player.blockPosition();
    int radius = cfg.scanRadius;
    if (queue.isEmpty() || ++age >= 100) {
      queue.clear();
      age = 0;
      partial = false;
      int cx = origin.getX() >> 4, cz = origin.getZ() >> 4, cr = (radius + 15) / 16;
      var jobs = new ArrayList<SectionPos>();
      int min = Math.max(level.getMinY(), cfg.minY) >> 4,
          max = Math.min(level.getMaxY(), cfg.maxY) >> 4;
      for (int x = cx - cr; x <= cx + cr; x++)
        for (int z = cz - cr; z <= cz + cr; z++) {
          if (level.getChunkSource().getChunk(x, z, ChunkStatus.FULL, false) == null) continue;
          for (int y = min; y <= max; y++) jobs.add(SectionPos.of(x, y, z));
        }
      jobs.sort(Comparator.comparingDouble(s -> s.center().distSqr(origin)));
      queue.addAll(jobs);
    }
    cache
        .entrySet()
        .removeIf(e -> {
          BlockPos p = BlockPos.of(e.getKey());
          return p.distSqr(origin) > (double) radius * radius
              || level
                      .getChunkSource()
                      .getChunk(p.getX() >> 4, p.getZ() >> 4, ChunkStatus.FULL, false)
                  == null
              || !cfg.targets.contains(
                  BuiltInRegistries.BLOCK.getKey(level.getBlockState(p).getBlock()).toString());
        });
    var furthestFirst =
        new PriorityQueue<Long>(
            Comparator.<Long>comparingDouble(k -> BlockPos.of(k).distSqr(origin)).reversed());
    furthestFirst.addAll(cache.keySet());
    int cells = 0, sections = 0;
    while (!queue.isEmpty() && sections++ < 32 && cells + 4096 <= cfg.scanBudget) {
      SectionPos s = queue.removeFirst();
      var chunk = level.getChunkSource().getChunk(s.x(), s.z(), ChunkStatus.FULL, false);
      if (chunk == null) continue;
      var section = chunk.getSection(level.getSectionIndex(s.minBlockY()));
      if (section.hasOnlyAir()
          || !section.maybeHas(state ->
              cfg.targets.contains(BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString())))
        continue;
      cells += 4096;
      var p = new BlockPos.MutableBlockPos();
      for (int y = 0; y < 16; y++)
        for (int z = 0; z < 16; z++)
          for (int x = 0; x < 16; x++) {
            int py = s.minBlockY() + y;
            if (py < cfg.minY || py > cfg.maxY) continue;
            var state = section.getBlockState(x, y, z);
            String id = BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString();
            if (!cfg.targets.contains(id)) continue;
            p.set(s.minBlockX() + x, py, s.minBlockZ() + z);
            if (p.distSqr(origin) > (double) radius * radius) continue;
            long packed = p.asLong();
            if (cache.containsKey(packed)) cache.put(packed, id);
            else if (cache.size() < cfg.maxTargets) {
              cache.put(packed, id);
              furthestFirst.add(packed);
            } else {
              partial = true;
              var furthest = furthestFirst.peek();
              if (furthest != null && BlockPos.of(furthest).distSqr(origin) > p.distSqr(origin)) {
                furthestFirst.poll();
                cache.remove(furthest);
                cache.put(packed, id);
                furthestFirst.add(packed);
              }
            }
          }
    }
    RustyClient.targets = Collections.unmodifiableMap(new LinkedHashMap<>(cache));
    RustyClient.partial = partial;
  }
}
