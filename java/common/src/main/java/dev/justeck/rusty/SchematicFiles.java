package dev.justeck.rusty;

import dev.justeck.rusty.core.Schematic;
import dev.justeck.rusty.core.SchematicCommit;
import dev.justeck.rusty.core.SchematicSave;
import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicLong;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.nbt.NbtAccounter;
import net.minecraft.nbt.NbtIo;

/** Native dialogs and bounded atomic file work never run on Minecraft's render/tick thread. */
public final class SchematicFiles {
  private static final ExecutorService WORKER =
      Executors.newSingleThreadExecutor(r -> {
        var t = new Thread(r, "Rusty schematic files");
        t.setDaemon(true);
        return t;
      });
  private static final AtomicLong GENERATION = new AtomicLong();
  public static volatile boolean busy;
  private static Capture capture;
  private static volatile Request activeRequest;
  public static volatile boolean commitMayFinish;

  private record Request(
      long generation, Object level, Object connection, String scope, SchematicCommit commit) {
    boolean current() {
      var mc = Minecraft.getInstance();
      return GENERATION.get() == generation
          && mc.level == level
          && mc.getConnection() == connection
          && RustyClient.scope().equals(scope);
    }
  }

  private static Request request() {
    cancel();
    var mc = Minecraft.getInstance();
    busy = true;
    var r =
        new Request(
            GENERATION.get(),
            mc.level,
            mc.getConnection(),
            RustyClient.scope(),
            new SchematicCommit());
    activeRequest = r;
    return r;
  }

  public static void cancel() {
    var old = activeRequest;
    if (old != null && !old.commit.cancel()) {
      commitMayFinish = true;
      RustyClient.notice = "Export commit already began; the chosen snapshot may finish saving";
    }
    activeRequest = null;
    GENERATION.incrementAndGet();
    RustySchematics.cancelImport();
    SchematicDialogs.cancel();
    capture = null;
    busy = false;
  }

  private static void notice(Request r, String message) {
    Minecraft.getInstance()
        .execute(() -> {
          if (r.current()) {
            busy = false;
            if (activeRequest == r) activeRequest = null;
            RustyClient.notice = message;
          }
        });
  }

  public static void load() {
    var r = request();
    WORKER.execute(() -> {
      if (!r.current()) return;
      try {
        var chosen = SchematicDialogs.choose(false, r::current);
        if (chosen == null) {
          notice(r, "Schematic chooser cancelled");
          return;
        }
        read(r, chosen.path());
      } catch (Exception e) {
        notice(r, "Schematic chooser/read failed: " + safe(e));
      }
    });
  }

  /** Same result path as the chooser; development fixtures do not simulate construction success. */
  public static void loadPath(Path path) {
    var r = request();
    WORKER.execute(() -> read(r, path));
  }

  private static void read(Request r, Path path) {
    if (!r.current()) return;
    try {
      if (!Files.isRegularFile(path) || Files.size(path) > Schematic.INPUT_LIMIT)
        throw new IOException("Input exceeds 32 MiB or is not a regular file");
      byte[] bytes;
      try (var in = Files.newInputStream(path)) {
        bytes = in.readNBytes(Schematic.INPUT_LIMIT + 1);
      }
      var document = Schematic.read(bytes);
      // Host NBT receives only strictly scanned, independently bounded expanded input.
      byte[] raw = Schematic.expanded(bytes);
      try (var in = new DataInputStream(new ByteArrayInputStream(raw))) {
        NbtIo.read(in, new NbtAccounter(Schematic.EXPANDED_LIMIT, 64));
      }
      Minecraft.getInstance()
          .execute(() -> {
            if (r.current()) {
              busy = false;
              RustySchematics.accept(document);
              RustyClient.notice = "Read " + basename(path) + "; validating registry states";
            }
          });
    } catch (Exception e) {
      notice(r, "Could not load " + basename(path) + ": " + safe(e));
    }
  }

  private static String basename(Path path) {
    String s = path.getFileName().toString().replaceAll("[\\r\\n]", " ");
    return s.substring(0, Math.min(100, s.length()));
  }

  private static String safe(Exception e) {
    String m =
        e instanceof FileSystemException f
            ? f.getReason()
            : e instanceof InvalidPathException p ? p.getReason() : e.getMessage();
    if (m == null) return e.getClass().getSimpleName();
    if (m.contains("Cannot run program")) return "Native desktop helper unavailable";
    m = m.replaceAll("[\\r\\n]", " ");
    return m.substring(0, Math.min(m.length(), 180));
  }

  public static void save() {
    beginCapture(null);
  }

  public static void savePath(Path path) {
    beginCapture(path);
  }

  private static final class Capture {
    Request r;
    BlockPos min;
    int x, y, z, n, index;
    long revision;
    Path path;
    List<Schematic.Cell> cells = new ArrayList<>();
  }

  private static void beginCapture(Path path) {
    var a = RustySchematics.first();
    var b = RustySchematics.second();
    if (a == null || b == null) {
      RustyClient.notice = "Set both export corners in this world first";
      return;
    }
    var c = new Capture();
    long x = Math.abs((long) a.getX() - b.getX()) + 1,
        y = Math.abs((long) a.getY() - b.getY()) + 1,
        z = Math.abs((long) a.getZ() - b.getZ()) + 1;
    if (x > 256 || y > 256 || z > 256 || x * y * z > Schematic.CELL_LIMIT) {
      RustyClient.notice = "Selection exceeds 256 per axis / 1,048,576 cells";
      return;
    }
    SchematicConstruction.stop("Export selected");
    c.r = request();
    c.min =
        new BlockPos(
            Math.min(a.getX(), b.getX()),
            Math.min(a.getY(), b.getY()),
            Math.min(a.getZ(), b.getZ()));
    c.x = (int) x;
    c.y = (int) y;
    c.z = (int) z;
    c.n = (int) (x * y * z);
    c.revision = SchematicConstruction.receivedRevision;
    c.path = path;
    capture = c;
  }

  public static void tick() {
    var c = capture;
    if (c == null) return;
    if (!c.r.current()) {
      cancel();
      return;
    }
    if (c.revision != SchematicConstruction.receivedRevision) {
      capture = null;
      notice(c.r, "Export cancelled: received terrain changed during capture");
      return;
    }
    var mc = Minecraft.getInstance();
    int end = Math.min(c.n, c.index + 2048);
    while (c.index < end) {
      int i = c.index++;
      var p = c.min.offset(i % c.x, i / (c.x * c.z), i / c.x % c.z);
      if (!SchematicConstruction.received(p)) {
        capture = null;
        notice(c.r, "Export stopped: unknown/unloaded/pending cells; nothing saved");
        return;
      }
      c.cells.add(
          new Schematic.Cell(
              new Schematic.Pos(i % c.x, i / (c.x * c.z), i / c.x % c.z),
              RustySchematics.describe(mc.level.getBlockState(p))));
    }
    if (c.index < c.n) {
      RustyClient.notice = "Capturing received states " + c.index + "/" + c.n;
      return;
    }
    capture = null;
    WORKER.execute(() ->
        write(c.r, new Schematic.Document(c.cells, 0, "Rusty states-only selection"), c.path));
  }

  private static void write(Request r, Schematic.Document document, Path supplied) {
    if (!r.current()) return;
    try {
      Path chosen = supplied;
      boolean overwrite = false;
      if (chosen == null) {
        var choice = SchematicDialogs.choose(true, r::current);
        if (choice == null) {
          notice(r, "Export chooser cancelled");
          return;
        }
        chosen = choice.path();
        overwrite = choice.overwrite();
      }
      chosen = chosen.toAbsolutePath().normalize();
      if (!chosen.getFileName().toString().toLowerCase(Locale.ROOT).endsWith(".schem"))
        chosen = chosen.resolveSibling(chosen.getFileName() + ".schem");
      if (!r.current()) return;
      // Capture destination identity immediately after the chosen path, before serialization.
      try (var save = new SchematicSave(chosen, overwrite)) {
        byte[] bytes =
            Schematic.sponge(
                document,
                net.minecraft.SharedConstants.getCurrentVersion().dataVersion().version());
        if (!r.current()) return;
        save.write(bytes);
        save.checkDestination();
        // All possibly blocking preparation is over. Cancellation and this claim race atomically;
        // the game thread never waits for the ensuing filesystem operation.
        if (!r.commit.claim(r::current)) return;
        save.commit(r.commit);
      }
      notice(
          r, "Saved states-only " + basename(chosen) + "; no block entities, entities or biomes");
    } catch (Exception e) {
      notice(r, "Selection was not saved: " + safe(e));
    }
  }
}
