package dev.justeck.rusty.core;

import java.io.*;
import java.util.*;
import java.util.zip.*;

/** Independent bounded named-state interchange. No legacy numeric-state conversion. */
public final class Schematic {
  public static final int INPUT_LIMIT = 32 * 1024 * 1024,
      EXPANDED_LIMIT = 64 * 1024 * 1024,
      CELL_LIMIT = 1048576,
      AXIS_LIMIT = 256,
      PALETTE_LIMIT = 65536;

  public record Pos(int x, int y, int z) {}

  public record State(String id, SortedMap<String, String> properties) {
    public State {
      if (!id.matches("[a-z0-9_.-]+:[a-z0-9_./-]+"))
        throw new IllegalArgumentException("Invalid block identifier");
      properties = Collections.unmodifiableSortedMap(new TreeMap<>(properties));
    }

    public static State parse(String text) {
      if (text == null || text.length() > 32767)
        throw new IllegalArgumentException("Invalid state string");
      int bracket = text.indexOf('[');
      String id = bracket < 0 ? text : text.substring(0, bracket);
      var props = new TreeMap<String, String>();
      if (bracket >= 0) {
        if (!text.endsWith("]")) throw new IllegalArgumentException("Invalid properties");
        for (String part :
            (bracket == text.length() - 2
                ? new String[0]
                : text.substring(bracket + 1, text.length() - 1).split(",", -1))) {
          String[] pair = part.split("=", -1);
          if (pair.length != 2
              || !pair[0].matches("[a-z0-9_]+")
              || !pair[1].matches("[a-z0-9_-]+")
              || props.putIfAbsent(pair[0], pair[1]) != null)
            throw new IllegalArgumentException("Invalid or duplicate property");
        }
      }
      return new State(id, props);
    }

    public String canonical() {
      return id
          + (properties.isEmpty()
              ? ""
              : "["
                  + String.join(
                      ",",
                      properties.entrySet().stream()
                          .map(e -> e.getKey() + "=" + e.getValue())
                          .toList())
                  + "]");
    }

    public boolean skipped() {
      return Set.of(
              "minecraft:air",
              "minecraft:cave_air",
              "minecraft:void_air",
              "minecraft:structure_void")
          .contains(id);
    }
  }

  public record Cell(Pos pos, State state) {}

  public record Document(List<Cell> cells, int detailCount, String format, List<State> palette) {
    public Document(List<Cell> cells, int detailCount, String format) {
      this(cells, detailCount, format, cells.stream().map(Cell::state).distinct().toList());
    }

    public Document {
      cells = List.copyOf(cells);
      palette = List.copyOf(palette);
      if (cells.size() > CELL_LIMIT || palette.size() > PALETTE_LIMIT)
        throw new IllegalArgumentException("Cell/global palette budget exceeded");
    }
  }

  public static Document read(byte[] input) throws IOException {
    try {
      byte[] raw = expanded(input);
      var in = new DataInputStream(new ByteArrayInputStream(raw));
      if (in.readUnsignedByte() != 10) throw invalid("Root is not a named compound");
      in.readUTF();
      var reader = new Reader(in);
      var root = compound(reader.payload(10, 0));
      if (in.available() != 0) throw invalid("Trailing NBT bytes");
      if (root.get("Schematic") instanceof Map<?, ?>) root = compound(root.get("Schematic"));
      if (root.containsKey("Regions")) return litematic(root);
      if (root.containsKey("Version")
          && (root.containsKey("Palette")
              || root.containsKey("Blocks") && root.get("Blocks") instanceof Map<?, ?>))
        return spongeRead(root);
      if (root.containsKey("size") && root.containsKey("blocks")) return vanilla(root);
      throw invalid("Unsupported format (legacy numeric .schematic needs conversion)");
    } catch (IllegalArgumentException | ArithmeticException e) {
      throw new IOException("Invalid schematic: " + e.getMessage(), e);
    }
  }

  public static byte[] expanded(byte[] input) throws IOException {
    if (input.length > INPUT_LIMIT) throw invalid("Input exceeds 32 MiB");
    if (input.length < 2 || input[0] != (byte) 31 || input[1] != (byte) 139) return input;
    var in = new DataInputStream(new ByteArrayInputStream(input));
    in.skipNBytes(2);
    if (in.readUnsignedByte() != 8) throw invalid("Invalid gzip method");
    int flags = in.readUnsignedByte();
    if ((flags & 0xe0) != 0) throw invalid("Invalid gzip flags");
    in.skipNBytes(6);
    if ((flags & 4) != 0) {
      int length = in.readUnsignedByte() | in.readUnsignedByte() << 8;
      in.skipNBytes(length);
    }
    for (int bit : new int[] {8, 16})
      if ((flags & bit) != 0) {
        int length = 0;
        while (in.readUnsignedByte() != 0)
          if (++length > 65535) throw invalid("Gzip header too long");
      }
    if ((flags & 2) != 0) {
      int headerLength = input.length - in.available();
      var crc = new CRC32();
      crc.update(input, 0, headerLength);
      int expected = in.readUnsignedByte() | in.readUnsignedByte() << 8;
      if ((crc.getValue() & 65535) != expected) throw invalid("Gzip header CRC mismatch");
    }
    int start = input.length - in.available();
    var inflater = new Inflater(true);
    var out = new ByteArrayOutputStream();
    var crc = new CRC32();
    try {
      inflater.setInput(input, start, input.length - start);
      byte[] buffer = new byte[8192];
      while (!inflater.finished()) {
        int n = inflater.inflate(buffer);
        if (n == 0) throw invalid("Truncated gzip stream");
        if ((long) out.size() + n > EXPANDED_LIMIT) throw invalid("Expanded data exceeds 64 MiB");
        out.write(buffer, 0, n);
        crc.update(buffer, 0, n);
      }
      int end = Math.toIntExact(start + inflater.getBytesRead());
      if (end + 8 != input.length) throw invalid("Trailing or concatenated gzip data");
      long expected = le32(input, end), size = le32(input, end + 4);
      if (expected != crc.getValue() || size != out.size()) throw invalid("Gzip checksum mismatch");
      return out.toByteArray();
    } catch (DataFormatException e) {
      throw new IOException("Invalid gzip", e);
    } finally {
      inflater.end();
    }
  }

  private static long le32(byte[] a, int p) {
    return (a[p] & 255L)
        | (a[p + 1] & 255L) << 8
        | (a[p + 2] & 255L) << 16
        | (a[p + 3] & 255L) << 24;
  }

  private static IOException invalid(String reason) {
    return new IOException(reason);
  }

  private static final class Reader {
    final DataInputStream in;
    int tags;
    long heapBudget;

    void account(long bytes) throws IOException {
      heapBudget = Math.addExact(heapBudget, bytes);
      if (heapBudget > EXPANDED_LIMIT) throw invalid("NBT allocation budget exceeded");
    }

    Reader(DataInputStream in) {
      this.in = in;
    }

    int length(int width) throws IOException {
      int n = in.readInt();
      if (n < 0 || (long) n * width > in.available() || n > EXPANDED_LIMIT / Math.max(1, width))
        throw invalid("Invalid NBT collection length");
      return n;
    }

    Object payload(int type, int depth) throws IOException {
      if (depth > 64 || ++tags > 2097152) throw invalid("NBT depth/tag budget exceeded");
      account(32);
      return switch (type) {
        case 1 -> in.readByte();
        case 2 -> in.readShort();
        case 3 -> in.readInt();
        case 4 -> in.readLong();
        case 5 -> in.readFloat();
        case 6 -> in.readDouble();
        case 7 -> {
          int n = length(1);
          account(n + 16L);
          byte[] a = new byte[n];
          in.readFully(a);
          yield a;
        }
        case 8 -> {
          String s = in.readUTF();
          account(2L * s.length() + 32);
          if (s.length() > 32767) throw invalid("NBT string too long");
          yield s;
        }
        case 9 -> {
          int t = in.readUnsignedByte(), n = length(1);
          if (n > CELL_LIMIT || t > 12 || t == 0 && n != 0) throw invalid("Invalid NBT list");
          account(8L * n + 24);
          var a = new ArrayList<Object>(n);
          for (int i = 0; i < n; i++) a.add(payload(t, depth + 1));
          yield a;
        }
        case 10 -> {
          var m = new LinkedHashMap<String, Object>();
          int t;
          while ((t = in.readUnsignedByte()) != 0) {
            String key = in.readUTF();
            account(2L * key.length() + 64);
            if (key.length() > 32767 || m.containsKey(key))
              throw invalid("Duplicate/invalid NBT key");
            m.put(key, payload(t, depth + 1));
          }
          yield m;
        }
        case 11 -> {
          int n = length(4);
          account(4L * n + 16);
          int[] a = new int[n];
          for (int i = 0; i < n; i++) a[i] = in.readInt();
          yield a;
        }
        case 12 -> {
          int n = length(8);
          account(8L * n + 16);
          long[] a = new long[n];
          for (int i = 0; i < n; i++) a[i] = in.readLong();
          yield a;
        }
        default -> throw invalid("Unknown NBT tag");
      };
    }
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> compound(Object o) throws IOException {
    if (!(o instanceof Map)) throw invalid("Expected compound");
    return (Map<String, Object>) o;
  }

  private static List<?> list(Object o) throws IOException {
    if (!(o instanceof List<?> l)) throw invalid("Expected list");
    return l;
  }

  private static int integer(Object o) throws IOException {
    if (!(o instanceof Integer i)) throw invalid("Expected integer");
    return i;
  }

  private static String string(Object o) throws IOException {
    if (!(o instanceof String s)) throw invalid("Expected string");
    return s;
  }

  private static int dimension(Object o) throws IOException {
    if (!(o instanceof Short s)) throw invalid("Expected unsigned short dimension");
    return s & 65535;
  }

  private static int volume(int x, int y, int z) throws IOException {
    if (x < 1
        || y < 1
        || z < 1
        || x > AXIS_LIMIT
        || y > AXIS_LIMIT
        || z > AXIS_LIMIT
        || (long) x * y * z > CELL_LIMIT) throw invalid("Dimension/cell budget exceeded");
    return x * y * z;
  }

  private static Pos xyz(Object value) throws IOException {
    var m = compound(value);
    return new Pos(integer(m.get("x")), integer(m.get("y")), integer(m.get("z")));
  }

  private static Pos triple(Object value) throws IOException {
    var l = list(value);
    if (l.size() != 3) throw invalid("Expected three coordinates");
    return new Pos(integer(l.get(0)), integer(l.get(1)), integer(l.get(2)));
  }

  private static State paletteState(Object value) throws IOException {
    var m = compound(value);
    var p = new TreeMap<String, String>();
    if (m.containsKey("Properties"))
      for (var e : compound(m.get("Properties")).entrySet()) {
        if (!e.getKey().matches("[a-z0-9_]+") || !string(e.getValue()).matches("[a-z0-9_-]+"))
          throw invalid("Invalid state property");
        p.put(e.getKey(), string(e.getValue()));
      }
    return new State(string(m.get("Name")), p);
  }

  private static List<State> palette(Object value) throws IOException {
    var l = list(value);
    if (l.isEmpty() || l.size() > PALETTE_LIMIT) throw invalid("Palette budget exceeded");
    var p = new ArrayList<State>();
    for (var v : l) p.add(paletteState(v));
    if (new HashSet<>(p).size() != p.size()) throw invalid("Duplicate palette state");
    return p;
  }

  private static int details(Map<String, Object> m, String... keys) throws IOException {
    int n = 0;
    for (String key : keys) if (m.containsKey(key)) n = Math.addExact(n, list(m.get(key)).size());
    return n;
  }

  private static Document finish(
      Map<Pos, State> cells, int details, String format, Collection<State> declared)
      throws IOException {
    if (cells.isEmpty() || cells.size() > CELL_LIMIT) throw invalid("Empty/oversized schematic");
    int minX = Integer.MAX_VALUE,
        minY = minX,
        minZ = minX,
        maxX = Integer.MIN_VALUE,
        maxY = maxX,
        maxZ = maxX;
    for (var p : cells.keySet()) {
      minX = Math.min(minX, p.x);
      minY = Math.min(minY, p.y);
      minZ = Math.min(minZ, p.z);
      maxX = Math.max(maxX, p.x);
      maxY = Math.max(maxY, p.y);
      maxZ = Math.max(maxZ, p.z);
    }
    if ((long) maxX - minX + 1 > AXIS_LIMIT
        || (long) maxY - minY + 1 > AXIS_LIMIT
        || (long) maxZ - minZ + 1 > AXIS_LIMIT) throw invalid("Overall axis budget exceeded");
    volume(
        Math.toIntExact((long) maxX - minX + 1),
        Math.toIntExact((long) maxY - minY + 1),
        Math.toIntExact((long) maxZ - minZ + 1));
    var out =
        cells.entrySet().stream()
            .map(e -> new Cell(e.getKey(), e.getValue()))
            .sorted(
                Comparator.comparingInt((Cell c) -> c.pos.y)
                    .thenComparingInt(c -> c.pos.z)
                    .thenComparingInt(c -> c.pos.x))
            .toList();
    return new Document(out, details, format, List.copyOf(new LinkedHashSet<>(declared)));
  }

  private static void add(Map<Pos, State> cells, Pos p, State s) throws IOException {
    if (cells.size() >= CELL_LIMIT || cells.putIfAbsent(p, s) != null)
      throw invalid("Overlapping/duplicate cells or cell budget exceeded");
  }

  private static Document spongeRead(Map<String, Object> root) throws IOException {
    int v = integer(root.get("Version"));
    if (v != 2 && v != 3) throw invalid("Sponge version must be 2 or 3");
    int x = dimension(root.get("Width")),
        y = dimension(root.get("Height")),
        z = dimension(root.get("Length")),
        n = volume(x, y, z);
    Pos offset = new Pos(0, 0, 0);
    if (root.containsKey("Offset")) {
      if (!(root.get("Offset") instanceof int[] a) || a.length != 3)
        throw invalid("Invalid offset");
      offset = new Pos(a[0], a[1], a[2]);
    }
    var blocks = v == 3 ? compound(root.get("Blocks")) : root;
    var p = new HashMap<Integer, State>();
    for (var e : compound(blocks.get("Palette")).entrySet()) {
      int id = integer(e.getValue());
      if (id < 0 || p.size() >= PALETTE_LIMIT || p.putIfAbsent(id, State.parse(e.getKey())) != null)
        throw invalid("Invalid/duplicate palette index");
    }
    if (p.isEmpty()) throw invalid("Empty palette");
    if (!(blocks.get(v == 3 ? "Data" : "BlockData") instanceof byte[] data))
      throw invalid("Missing block data");
    int cursor = 0;
    var cells = new HashMap<Pos, State>();
    for (int i = 0; i < n; i++) {
      int id = 0, bytes = 0, b;
      do {
        if (cursor >= data.length || bytes == 5) throw invalid("Truncated/overlong varint");
        b = data[cursor++] & 255;
        if (bytes == 4 && (b & 0xf8) != 0) throw invalid("Varint overflow");
        id |= (b & 127) << (bytes++ * 7);
      } while ((b & 128) != 0);
      if (bytes > 1 && (b & 127) == 0) throw invalid("Noncanonical varint");
      var s = p.get(id);
      if (s == null) throw invalid("Unknown palette index");
      add(
          cells,
          new Pos(
              Math.addExact(offset.x, i % x),
              Math.addExact(offset.y, i / (x * z)),
              Math.addExact(offset.z, i / x % z)),
          s);
    }
    if (cursor != data.length) throw invalid("Trailing block data");
    return finish(
        cells,
        details(root, "Entities") + details(blocks, "BlockEntities"),
        "Sponge v" + v,
        p.values());
  }

  private static Document litematic(Map<String, Object> root) throws IOException {
    int v = integer(root.get("Version"));
    if (v < 4 || v > 7) throw invalid("Litematic version must be 4–7 with named states");
    var regions = compound(root.get("Regions"));
    if (regions.isEmpty() || regions.size() > PALETTE_LIMIT)
      throw invalid("Region budget exceeded");
    var cells = new HashMap<Pos, State>();
    int detail = 0;
    var declared = new LinkedHashSet<State>();
    for (var value : regions.values()) {
      var r = compound(value);
      Pos p = xyz(r.get("Position")), size = xyz(r.get("Size"));
      int x = Math.absExact(size.x),
          y = Math.absExact(size.y),
          z = Math.absExact(size.z),
          n = volume(x, y, z);
      int
          ox =
              Math.min(p.x, Math.addExact(p.x, Math.subtractExact(size.x, Integer.signum(size.x)))),
          oy =
              Math.min(p.y, Math.addExact(p.y, Math.subtractExact(size.y, Integer.signum(size.y)))),
          oz =
              Math.min(p.z, Math.addExact(p.z, Math.subtractExact(size.z, Integer.signum(size.z))));
      var pal = palette(r.get("BlockStatePalette"));
      declared.addAll(pal);
      if (declared.size() > PALETTE_LIMIT) throw invalid("Global declared palette budget exceeded");
      int bits = Math.max(2, 32 - Integer.numberOfLeadingZeros(pal.size() - 1));
      if (!(r.get("BlockStates") instanceof long[] words)
          || words.length != ((long) n * bits + 63) / 64)
        throw invalid("Invalid packed state length");
      long mask = (1L << bits) - 1;
      for (int i = 0; i < n; i++) {
        int bit = i * bits, word = bit / 64, shift = bit % 64;
        long id = words[word] >>> shift;
        if (shift + bits > 64) id |= words[word + 1] << (64 - shift);
        int index = (int) (id & mask);
        if (index >= pal.size()) throw invalid("Unknown palette index");
        add(
            cells,
            new Pos(
                Math.addExact(ox, i % x),
                Math.addExact(oy, i / (x * z)),
                Math.addExact(oz, i / x % z)),
            pal.get(index));
      }
      detail =
          Math.addExact(
              detail,
              details(r, "TileEntities", "Entities", "PendingBlockTicks", "PendingFluidTicks"));
    }
    return finish(cells, detail, "Litematic v" + v, declared);
  }

  private static Document vanilla(Map<String, Object> root) throws IOException {
    Pos size = triple(root.get("size"));
    volume(size.x, size.y, size.z);
    Object chosen = root.get("palette");
    if (chosen == null) {
      var variants = list(root.get("palettes"));
      if (variants.isEmpty()) throw invalid("Empty palette variants");
      chosen = variants.getFirst();
    }
    var pal = palette(chosen);
    var declared = new LinkedHashSet<State>(pal);
    if (root.containsKey("palettes"))
      for (var variant : list(root.get("palettes"))) {
        declared.addAll(palette(variant));
        if (declared.size() > PALETTE_LIMIT)
          throw invalid("Global declared palette budget exceeded");
      }
    var cells = new HashMap<Pos, State>();
    int detail = details(root, "entities");
    for (var value : list(root.get("blocks"))) {
      var b = compound(value);
      Pos p = triple(b.get("pos"));
      if (p.x < 0 || p.y < 0 || p.z < 0 || p.x >= size.x || p.y >= size.y || p.z >= size.z)
        throw invalid("Block position outside size");
      int id = integer(b.get("state"));
      if (id < 0 || id >= pal.size()) throw invalid("Unknown palette index");
      add(cells, p, pal.get(id));
      if (b.containsKey("nbt")) {
        compound(b.get("nbt"));
        detail++;
      }
    }
    return finish(
        cells, detail, "Vanilla structure (palette variant 1; sparse gaps untouched)", declared);
  }

  public static byte[] sponge(Document doc) throws IOException {
    return sponge(doc, 0);
  }

  public static byte[] sponge(Document doc, int dataVersion) throws IOException {
    if (doc.cells.isEmpty()) throw invalid("Nothing to export");
    int minX = doc.cells.stream().mapToInt(c -> c.pos.x).min().orElseThrow(),
        minY = doc.cells.stream().mapToInt(c -> c.pos.y).min().orElseThrow(),
        minZ = doc.cells.stream().mapToInt(c -> c.pos.z).min().orElseThrow();
    int
        x =
            Math.toIntExact(
                (long) doc.cells.stream().mapToInt(c -> c.pos.x).max().orElseThrow() - minX + 1),
        y =
            Math.toIntExact(
                (long) doc.cells.stream().mapToInt(c -> c.pos.y).max().orElseThrow() - minY + 1),
        z =
            Math.toIntExact(
                (long) doc.cells.stream().mapToInt(c -> c.pos.z).max().orElseThrow() - minZ + 1),
        n = volume(x, y, z);
    var cells = new HashMap<Pos, State>();
    for (var c : doc.cells) add(cells, c.pos, c.state);
    var palette = new LinkedHashMap<String, Integer>();
    var data = new ByteArrayOutputStream();
    for (int i = 0; i < n; i++) {
      String s =
          cells
              .getOrDefault(
                  new Pos(minX + i % x, minY + i / (x * z), minZ + i / x % z),
                  State.parse("minecraft:structure_void"))
              .canonical();
      int id = palette.computeIfAbsent(s, k -> palette.size());
      if (palette.size() > PALETTE_LIMIT) throw invalid("Palette budget exceeded");
      do {
        int b = id & 127;
        id >>>= 7;
        data.write(b | (id == 0 ? 0 : 128));
      } while (id != 0);
    }
    var raw = new ByteArrayOutputStream();
    try (var d = new DataOutputStream(raw)) {
      d.writeByte(10);
      d.writeUTF("");
      tag(d, 10, "Schematic");
      tag(d, 3, "Version");
      d.writeInt(3);
      tag(d, 3, "DataVersion");
      d.writeInt(dataVersion);
      tag(d, 2, "Width");
      d.writeShort(x);
      tag(d, 2, "Height");
      d.writeShort(y);
      tag(d, 2, "Length");
      d.writeShort(z);
      tag(d, 11, "Offset");
      d.writeInt(3);
      d.writeInt(minX);
      d.writeInt(minY);
      d.writeInt(minZ);
      tag(d, 10, "Metadata");
      tag(d, 8, "Name");
      d.writeUTF("Rusty states-only selection (no block entities, entities or biomes)");
      d.writeByte(0);
      tag(d, 10, "Blocks");
      tag(d, 10, "Palette");
      for (var e : palette.entrySet()) {
        tag(d, 3, e.getKey());
        d.writeInt(e.getValue());
      }
      d.writeByte(0);
      tag(d, 7, "Data");
      d.writeInt(data.size());
      data.writeTo(d);
      d.writeByte(0);
      d.writeByte(0);
      d.writeByte(0);
    }
    var out = new ByteArrayOutputStream();
    try (var gzip = new GZIPOutputStream(out)) {
      gzip.write(raw.toByteArray());
    }
    if (out.size() > INPUT_LIMIT) throw invalid("Export exceeds input budget");
    return out.toByteArray();
  }

  private static void tag(DataOutputStream d, int type, String name) throws IOException {
    d.writeByte(type);
    d.writeUTF(name);
  }

  public static Pos transform(Pos p, int turns, boolean mirror) {
    int x = mirror ? Math.negateExact(p.x) : p.x, z = p.z;
    for (int i = 0; i < Math.floorMod(turns, 4); i++) {
      int old = x;
      x = Math.negateExact(z);
      z = old;
    }
    return new Pos(x, p.y, z);
  }

  public static int materialUnits(State s) {
    if (s.skipped()
        || "upper".equals(s.properties.get("half"))
        || "head".equals(s.properties.get("part"))) return 0;
    if ("double".equals(s.properties.get("type")) && s.id.endsWith("_slab")) return 2;
    for (String key : new String[] {"layers", "candles", "eggs", "pickles"})
      if (s.properties.containsKey(key)) {
        try {
          int count = Integer.parseInt(s.properties.get(key));
          int maximum = key.equals("layers") ? 8 : 4;
          return count >= 1 && count <= maximum ? count : 1;
        } catch (NumberFormatException e) {
          return 1;
        }
      }
    return 1;
  }

  public static boolean predictedWritesMatch(Map<Pos, State> planned, Map<Pos, State> writes) {
    return !writes.isEmpty()
        && writes.entrySet().stream().allMatch(e -> e.getValue().equals(planned.get(e.getKey())));
  }
}
