package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import java.io.*;
import java.util.*;
import java.util.zip.GZIPOutputStream;
import org.junit.jupiter.api.Test;

class SchematicTest {
  static byte[] nbt(Map<String, Object> root) throws IOException {
    var b = new ByteArrayOutputStream();
    var d = new DataOutputStream(b);
    d.writeByte(10);
    d.writeUTF("");
    compound(d, root);
    return b.toByteArray();
  }

  static int type(Object o) {
    if (o instanceof Integer) return 3;
    if (o instanceof Short) return 2;
    if (o instanceof String) return 8;
    if (o instanceof byte[]) return 7;
    if (o instanceof int[]) return 11;
    if (o instanceof long[]) return 12;
    if (o instanceof List) return 9;
    if (o instanceof Map) return 10;
    throw new AssertionError(o);
  }

  static void compound(DataOutputStream d, Map<String, Object> m) throws IOException {
    for (var e : m.entrySet()) {
      d.writeByte(type(e.getValue()));
      d.writeUTF(e.getKey());
      payload(d, e.getValue());
    }
    d.writeByte(0);
  }

  @SuppressWarnings("unchecked")
  static void payload(DataOutputStream d, Object o) throws IOException {
    switch (type(o)) {
      case 2 -> d.writeShort((Short) o);
      case 3 -> d.writeInt((Integer) o);
      case 8 -> d.writeUTF((String) o);
      case 7 -> {
        var a = (byte[]) o;
        d.writeInt(a.length);
        d.write(a);
      }
      case 11 -> {
        var a = (int[]) o;
        d.writeInt(a.length);
        for (int x : a) d.writeInt(x);
      }
      case 12 -> {
        var a = (long[]) o;
        d.writeInt(a.length);
        for (long x : a) d.writeLong(x);
      }
      case 10 -> compound(d, (Map<String, Object>) o);
      case 9 -> {
        var a = (List<?>) o;
        d.writeByte(a.isEmpty() ? 0 : type(a.getFirst()));
        d.writeInt(a.size());
        for (var x : a) payload(d, x);
      }
      default -> throw new AssertionError();
    }
  }

  static Map<String, Object> sponge(int version, byte[] data) {
    var m = new LinkedHashMap<String, Object>();
    m.put("Version", version);
    m.put("Width", (short) 2);
    m.put("Height", (short) 1);
    m.put("Length", (short) 1);
    var palette = Map.<String, Object>of("minecraft:air", 0, "minecraft:stone", 1);
    if (version == 2) {
      m.put("Palette", palette);
      m.put("BlockData", data);
    } else m.put("Blocks", Map.of("Palette", palette, "Data", data));
    return m;
  }

  static Schematic.State state(String s) {
    return Schematic.State.parse(s);
  }

  @Test
  void emptyPropertiesNormalizeInIndependentSpongeBytes() throws Exception {
    for (int version : new int[] {2, 3}) {
      var root = sponge(version, new byte[] {0, 1});
      var palette = Map.<String, Object>of("minecraft:air[]", 0, "minecraft:stone[]", 1);
      if (version == 2) root.put("Palette", palette);
      else root.put("Blocks", Map.of("Palette", palette, "Data", new byte[] {0, 1}));
      var d = Schematic.read(nbt(version == 3 ? Map.of("Schematic", root) : root));
      assertEquals(state("minecraft:air"), d.cells().get(0).state());
      assertEquals(state("minecraft:stone"), d.cells().get(1).state());
      assertEquals("minecraft:stone", d.cells().get(1).state().canonical());
    }
  }

  @Test
  void malformedPropertyEntriesRemainRejected() throws Exception {
    for (String suffix :
        List.of(
            "[,]",
            "[axis]",
            "[axis=]",
            "[=y]",
            "[axis=y,]",
            "[axis=y,axis=x]",
            "[[]",
            "[]]",
            "[axis=y",
            "[axis=y][axis=x]")) {
      for (int version : new int[] {2, 3}) {
        var root = sponge(version, new byte[] {0, 0});
        var palette = Map.of("minecraft:stone" + suffix, 0);
        if (version == 2) root.put("Palette", palette);
        else root.put("Blocks", Map.of("Palette", palette, "Data", new byte[] {0, 0}));
        assertThrows(
            IOException.class,
            () -> Schematic.read(nbt(version == 3 ? Map.of("Schematic", root) : root)),
            suffix);
      }
    }
  }

  @Test
  void readsSpongeTwoAndThree() throws Exception {
    for (int v : new int[] {2, 3}) {
      var r = sponge(v, new byte[] {1, 0});
      var doc = Schematic.read(nbt(v == 3 ? Map.of("Schematic", r) : r));
      assertEquals(2, doc.cells().size());
      assertEquals("minecraft:stone", doc.cells().getFirst().state().id());
    }
  }

  @Test
  void rejectsMissingPaletteIndex() throws Exception {
    assertThrows(IOException.class, () -> Schematic.read(nbt(sponge(2, new byte[] {2, 0}))));
  }

  @Test
  void rejectsTruncatedOverlongAndTrailingVarints() throws Exception {
    for (byte[] bad :
        new byte[][] {
          {(byte) 128},
          {(byte) 128, 0, 0},
          {1, 0, 0},
          {(byte) 255, (byte) 255, (byte) 255, (byte) 255, 127, 0}
        }) assertThrows(IOException.class, () -> Schematic.read(nbt(sponge(2, bad))));
  }

  @Test
  void rejectsDuplicatePaletteIdsAndOversizedDimensions() throws Exception {
    var m = sponge(2, new byte[] {0, 0});
    m.put("Palette", Map.of("minecraft:air", 0, "minecraft:stone", 0));
    assertThrows(IOException.class, () -> Schematic.read(nbt(m)));
    m.put("Width", (short) 257);
    assertThrows(IOException.class, () -> Schematic.read(nbt(m)));
  }

  @Test
  void canonicalStateRejectsDuplicateOrInvalidProperties() {
    assertEquals("minecraft:oak_log[axis=y]", state("minecraft:oak_log[axis=y]").canonical());
    assertThrows(IllegalArgumentException.class, () -> state("minecraft:stone[a=x,a=y]"));
    assertThrows(IllegalArgumentException.class, () -> state("bad name"));
  }

  @Test
  void readsSignedLitematicExtentsAndCrossWordIndices() throws Exception {
    var pal = new ArrayList<Map<String, Object>>();
    for (int i = 0; i < 17; i++) pal.add(Map.of("Name", "test:block_" + i));
    int n = 26, bits = 5;
    long[] words = new long[(n * bits + 63) / 64];
    for (int i = 0; i < n; i++) {
      int index = i % 17, at = i * bits;
      words[at / 64] |= (long) index << (at % 64);
      if (at % 64 + bits > 64) words[at / 64 + 1] |= (long) index >>> (64 - at % 64);
    }
    var region =
        Map.<String, Object>of(
            "Position",
            Map.of("x", 5, "y", 0, "z", 0),
            "Size",
            Map.of("x", -26, "y", 1, "z", 1),
            "BlockStatePalette",
            pal,
            "BlockStates",
            words);
    for (int v = 4; v <= 7; v++) {
      var doc = Schematic.read(nbt(Map.of("Version", v, "Regions", Map.of("a", region))));
      assertEquals(-20, doc.cells().getFirst().pos().x());
      assertEquals("test:block_13", doc.cells().get(13).state().id());
    }
  }

  @Test
  void rejectsLitematicOverlap() throws Exception {
    var r =
        Map.<String, Object>of(
            "Position",
            Map.of("x", 0, "y", 0, "z", 0),
            "Size",
            Map.of("x", 1, "y", 1, "z", 1),
            "BlockStatePalette",
            List.of(Map.of("Name", "minecraft:stone")),
            "BlockStates",
            new long[] {0});
    assertThrows(
        IOException.class,
        () -> Schematic.read(nbt(Map.of("Version", 7, "Regions", Map.of("a", r, "b", r)))));
  }

  @Test
  void vanillaIsSparseAndCountsDetails() throws Exception {
    var doc =
        Schematic.read(
            nbt(
                Map.of(
                    "size",
                    List.of(2, 1, 1),
                    "palette",
                    List.of(Map.of("Name", "minecraft:stone")),
                    "blocks",
                    List.of(
                        Map.of("pos", List.of(1, 0, 0), "state", 0, "nbt", Map.of("id", "chest"))),
                    "entities",
                    List.of(Map.of("x", 1)))));
    assertEquals(1, doc.cells().size());
    assertEquals(2, doc.detailCount());
    assertEquals(1, doc.cells().getFirst().pos().x());
  }

  @Test
  void rejectsVanillaDuplicateAndOutOfBounds() throws Exception {
    var block = Map.<String, Object>of("pos", List.of(0, 0, 0), "state", 0);
    var m =
        new LinkedHashMap<String, Object>(
            Map.of(
                "size",
                List.of(1, 1, 1),
                "palette",
                List.of(Map.of("Name", "minecraft:stone")),
                "blocks",
                List.of(block, block)));
    assertThrows(IOException.class, () -> Schematic.read(nbt(m)));
    m.put("blocks", List.of(Map.of("pos", List.of(1, 0, 0), "state", 0)));
    assertThrows(IOException.class, () -> Schematic.read(nbt(m)));
  }

  @Test
  void rejectsMalformedNbtAndLegacy() throws Exception {
    assertThrows(IOException.class, () -> Schematic.read(new byte[] {10, 0, 0, 3, 0}));
    assertThrows(
        IOException.class,
        () -> Schematic.read(nbt(Map.of("Blocks", new byte[] {1}, "Data", new byte[] {0}))));
    var b = new ByteArrayOutputStream();
    var d = new DataOutputStream(b);
    d.writeByte(10);
    d.writeUTF("");
    for (int i = 0; i < 2; i++) {
      d.writeByte(3);
      d.writeUTF("Version");
      d.writeInt(2);
    }
    d.writeByte(0);
    assertThrows(IOException.class, () -> Schematic.read(b.toByteArray()));
  }

  @Test
  void gzipAndRoundTripPreserveOffsetAndStates() throws Exception {
    var input = nbt(sponge(2, new byte[] {1, 0}));
    var out = new ByteArrayOutputStream();
    try (var gzip = new GZIPOutputStream(out)) {
      gzip.write(input);
    }
    var doc = Schematic.read(out.toByteArray());
    assertEquals(2, doc.cells().size());
    assertEquals("minecraft:stone", doc.cells().getFirst().state().id());
    assertEquals(doc.cells(), Schematic.read(Schematic.sponge(doc)).cells());
  }

  @Test
  void sparseBoundingVolumeIsStillBounded() throws Exception {
    var r =
        new LinkedHashMap<String, Object>(
            Map.of(
                "Position",
                Map.of("x", 0, "y", 0, "z", 0),
                "Size",
                Map.of("x", 1, "y", 1, "z", 1),
                "BlockStatePalette",
                List.of(Map.of("Name", "minecraft:stone")),
                "BlockStates",
                new long[] {0}));
    var far = new LinkedHashMap<>(r);
    far.put("Position", Map.of("x", 255, "y", 255, "z", 255));
    assertThrows(
        IOException.class,
        () -> Schematic.read(nbt(Map.of("Version", 7, "Regions", Map.of("a", r, "b", far)))));
  }

  @Test
  void rejectsGzipTrailingConcatenatedAndBadChecksum() throws Exception {
    var b = new ByteArrayOutputStream();
    try (var g = new GZIPOutputStream(b)) {
      g.write(nbt(sponge(2, new byte[] {1, 0})));
    }
    byte[] bytes = b.toByteArray();
    var trailing = Arrays.copyOf(bytes, bytes.length + 1);
    assertThrows(IOException.class, () -> Schematic.read(trailing));
    var concat = new byte[bytes.length * 2];
    System.arraycopy(bytes, 0, concat, 0, bytes.length);
    System.arraycopy(bytes, 0, concat, bytes.length, bytes.length);
    assertThrows(IOException.class, () -> Schematic.read(concat));
    bytes[bytes.length - 8] ^= 1;
    assertThrows(IOException.class, () -> Schematic.read(bytes));
  }

  @Test
  void rejectsDeepNbtBeforeSchema() throws Exception {
    Map<String, Object> root = Map.of("x", 1);
    for (int i = 0; i < 66; i++) root = Map.of("nested", root);
    byte[] bytes = nbt(root);
    assertThrows(IOException.class, () -> Schematic.read(bytes));
  }

  @Test
  void transformUsesQuarterTurnsAndMirror() {
    assertEquals(
        new Schematic.Pos(-3, 2, -1), Schematic.transform(new Schematic.Pos(1, 2, 3), 1, true));
    assertEquals(
        new Schematic.Pos(1, 2, 3), Schematic.transform(new Schematic.Pos(1, 2, 3), 4, false));
  }

  @Test
  void countsDoubleAndCoupledMaterials() {
    assertEquals(2, Schematic.materialUnits(state("minecraft:stone_slab[type=double]")));
    assertEquals(0, Schematic.materialUnits(state("minecraft:oak_door[half=upper]")));
    assertEquals(0, Schematic.materialUnits(state("minecraft:red_bed[part=head]")));
    assertEquals(4, Schematic.materialUnits(state("minecraft:snow[layers=4]")));
  }

  @Test
  void predictionMustContainEveryExactPlannedWrite() {
    var a = new Schematic.Pos(0, 0, 0);
    var b = new Schematic.Pos(0, 1, 0);
    var s = state("minecraft:stone");
    assertTrue(Schematic.predictedWritesMatch(Map.of(a, s, b, s), Map.of(a, s, b, s)));
    assertFalse(Schematic.predictedWritesMatch(Map.of(a, s), Map.of(a, s, b, s)));
    assertFalse(Schematic.predictedWritesMatch(Map.of(a, s), Map.of(a, state("minecraft:dirt"))));
    assertFalse(Schematic.predictedWritesMatch(Map.of(), Map.of()));
  }

  @Test
  void unusedDeclaredStatesRemainAvailableForStrictRegistryValidation() throws Exception {
    var root = sponge(2, new byte[] {1, 1});
    root.put("Palette", Map.of("minecraft:stone", 1, "invalid:unused_unknown", 7));
    var d = Schematic.read(nbt(root));
    assertEquals(2, d.palette().size());
    assertTrue(d.palette().contains(state("invalid:unused_unknown")));
    var vanilla =
        Map.<String, Object>of(
            "size",
            List.of(1, 1, 1),
            "palette",
            List.of(Map.of("Name", "minecraft:stone"), Map.of("Name", "invalid:unused_unknown")),
            "blocks",
            List.of(Map.of("pos", List.of(0, 0, 0), "state", 0)));
    assertEquals(2, Schematic.read(nbt(vanilla)).palette().size());
    var region =
        Map.<String, Object>of(
            "Position",
            Map.of("x", 0, "y", 0, "z", 0),
            "Size",
            Map.of("x", 1, "y", 1, "z", 1),
            "BlockStatePalette",
            List.of(Map.of("Name", "minecraft:stone"), Map.of("Name", "invalid:unused_unknown")),
            "BlockStates",
            new long[] {0});
    assertEquals(
        2,
        Schematic.read(nbt(Map.of("Version", 7, "Regions", Map.of("a", region)))).palette().size());
  }
}
