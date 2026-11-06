package dev.justeck.rusty.qa;

import dev.justeck.rusty.*;
import java.nio.file.*;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.client.gui.components.*;
import net.minecraft.client.input.MouseButtonInfo;

/** Real development-world widgets and framebuffer evidence; never opens a file chooser. */
public final class SchematicUiEvidence {
  private static final Set<String> REQUIRED =
      Set.of(
          "Load schematic…",
          "Materials / exact states",
          "Copy all materials",
          "Start construction (ordinary walking)",
          "Export: set first corner",
          "Export: set second corner",
          "Save states-only selection…");
  private static final Set<String> seen = new TreeSet<>();
  private static final List<Map<String, Object>> captures = new ArrayList<>();
  private static String runId, callbackMessage;
  private static int age, plannerPage, materialPage, materialPages, initialUses;
  private static int toastWaitTicks;
  private static Map<String, Object> displayedTotals = Map.of();
  private static Map<String, Object> clipboardComparison = Map.of();
  private static boolean materials, copied, captured, complete;
  private static volatile boolean captureComplete;
  private static Path pending;
  private static Map<net.minecraft.core.BlockPos, net.minecraft.world.level.block.state.BlockState>
      targets;

  private static Minecraft mc() {
    return Minecraft.getInstance();
  }

  private static void require(boolean value, String reason) {
    if (!value) throw new AssertionError("Schematic UI: " + reason);
  }

  private static Object field(Object object, String name) {
    try {
      var field = object.getClass().getDeclaredField(name);
      field.setAccessible(true);
      return field.get(object);
    } catch (ReflectiveOperationException e) {
      throw new AssertionError(e);
    }
  }

  private static void set(Object object, String name, Object value) {
    try {
      var field = object.getClass().getDeclaredField(name);
      field.setAccessible(true);
      field.set(object, value);
    } catch (ReflectiveOperationException e) {
      throw new AssertionError(e);
    }
  }

  private static List<Button> buttons() {
    return mc().gui.screen().children().stream()
        .filter(c -> c instanceof Button)
        .map(c -> (Button) c)
        .filter(b -> b.visible)
        .toList();
  }

  private static Set<String> labels() {
    var labels = new TreeSet<String>();
    buttons().forEach(b -> labels.add(b.getMessage().getString()));
    return labels;
  }

  private static void press(String label) {
    var button =
        buttons().stream()
            .filter(b -> b.getMessage().getString().equals(label))
            .max(Comparator.comparingInt(Button::getX))
            .orElseThrow(() -> new AssertionError("Missing real schematic UI button: " + label));
    require(button.active, "Inactive button " + label);
    button.onPress(new MouseButtonInfo(0, 0));
  }

  public static void begin() {
    runId =
        System.getProperty("rusty.edition", "java")
            + "-"
            + System.currentTimeMillis()
            + "-"
            + UUID.randomUUID().toString().substring(0, 8);
    initialUses = SchematicLookEvidence.uses;
    targets = Map.copyOf(RustySchematics.targets());
    require(targets.size() == 12 && RustySchematics.ready(), "Expected ready twelve-cell plan");
    var screen = new RustyScreen();
    set(screen, "selected", "build_planner");
    set(screen, "query", "Build Planner");
    mc().gui.setScreen(screen);
  }

  private static void paused() {
    require(!SchematicConstruction.active(), "UI or capture activated construction");
    require(SchematicLookEvidence.uses == initialUses, "UI dispatched a block use");
    require(
        RustySchematics.ready() && RustySchematics.targets().equals(targets),
        "UI changed the accepted plan or transform");
    require(!SchematicFiles.busy, "UI unexpectedly opened a file operation");
  }

  private static void fit() {
    var screen = mc().gui.screen();
    double scale = screen instanceof RustyScreen ? (double) field(screen, "scale") : 1;
    int width = mc().getWindow().getGuiScaledWidth(),
        height = mc().getWindow().getGuiScaledHeight();
    for (var button : buttons()) {
      require(
          button.getX() * scale >= 0
              && button.getY() * scale >= 0
              && (button.getX() + button.getWidth()) * scale <= width + 1
              && (button.getY() + button.getHeight()) * scale <= height + 1,
          "Actual widget outside rendered panel: " + button.getMessage().getString());
      require(
          mc().font.width(button.getMessage()) <= button.getWidth() - 4,
          "Actual widget text would be clipped: " + button.getMessage().getString());
    }
  }

  private static void capture(String name) {
    fit();
    String filename = "rusty-schematic-" + runId + "-" + name + ".png";
    pending = mc().gameDirectory.toPath().resolve("screenshots").resolve(filename);
    require(!Files.exists(pending), "Refusing to replace an earlier screenshot");
    captureComplete = false;
    captured = true;
    captures.add(
        new LinkedHashMap<>(
            Map.of(
                "path",
                pending.toAbsolutePath().toString(),
                "screen",
                mc().gui.screen().getClass().getSimpleName(),
                "page",
                name,
                "buttons",
                labels(),
                "provenance",
                "Minecraft Screenshot.grab live framebuffer")));
    Screenshot.grab(
        mc().gameDirectory,
        filename,
        mc().gameRenderer.mainRenderTarget(),
        1,
        message -> {
          callbackMessage = message.getString();
          captureComplete = true;
        });
  }

  private static void saved() {
    try {
      var bytes = Files.readAllBytes(pending);
      require(
          bytes.length > 1024
              && bytes[0] == (byte) 137
              && bytes[1] == 80
              && bytes[2] == 78
              && bytes[3] == 71,
          "Capture callback did not save a real PNG");
      int width = java.nio.ByteBuffer.wrap(bytes, 16, 4).getInt();
      int height = java.nio.ByteBuffer.wrap(bytes, 20, 4).getInt();
      require(width >= 640 && height >= 360, "Unreadably small framebuffer");
      captures.getLast().put("callback", callbackMessage);
      captures.getLast().put("width", width);
      captures.getLast().put("height", height);
      captures.getLast().put("bytes", bytes.length);
      captures.getLast().put("saved", true);
    } catch (java.io.IOException e) {
      throw new AssertionError("Capture save failed", e);
    }
    pending = null;
  }

  public static void inspectMaterialRows(List<String> lines) {
    int itemsStart = lines.indexOf("Items (quantity; stacks + remainder; have; missing)");
    int statesStart = lines.indexOf("Exact block-state totals (including air/skipped cells)");
    int manualStart = lines.indexOf("Manual requirements");
    require(itemsStart >= 0 && statesStart > itemsStart, "Displayed item/exact-state sections");
    int statesEnd = manualStart < 0 ? lines.size() : manualStart;
    require(statesEnd > statesStart, "Manual section follows exact-state section");
    var itemRows =
        lines.subList(itemsStart + 1, statesStart).stream().filter(s -> !s.isBlank()).toList();
    var itemIds =
        itemRows.stream().map(s -> s.substring(s.indexOf('[') + 1, s.indexOf(']'))).toList();
    var itemTotals = new TreeMap<String, Integer>();
    for (String row : itemRows) {
      int close = row.indexOf(']'), end = row.indexOf(';', close);
      require(close > 0 && end > close, "Readable displayed item quantity: " + row);
      String id = row.substring(row.indexOf('[') + 1, close);
      require(
          itemTotals.put(id, Integer.parseInt(row.substring(close + 3, end))) == null,
          "No duplicate displayed item: " + id);
    }
    require(
        itemIds.size() == 8 && itemIds.equals(itemIds.stream().sorted().toList()),
        "Displayed eight material IDs must be sorted");
    require(
        itemTotals.equals(
            Map.of(
                "minecraft:cobblestone",
                2,
                "minecraft:glass",
                1,
                "minecraft:oak_door",
                1,
                "minecraft:oak_log",
                1,
                "minecraft:oak_planks",
                2,
                "minecraft:oak_slab",
                1,
                "minecraft:oak_stairs",
                1,
                "minecraft:red_bed",
                1)),
        "Independent fixture has exactly eight material IDs and ten placement items");
    var states =
        lines.subList(statesStart + 1, statesEnd).stream().filter(s -> !s.isBlank()).toList();
    var stateTotals = new TreeMap<String, Integer>();
    for (String row : states) {
      int split = row.lastIndexOf(": ");
      require(split > 0, "Readable displayed state count: " + row);
      String state = row.substring(0, split);
      require(
          dev.justeck.rusty.core.Schematic.State.parse(state).canonical().equals(state),
          "Canonical displayed exact state: " + state);
      int count = Integer.parseInt(row.substring(split + 2));
      require(
          count > 0 && stateTotals.put(state, count) == null, "Positive unique exact-state count");
    }
    int all = stateTotals.values().stream().mapToInt(Integer::intValue).sum();
    int air = stateTotals.getOrDefault("minecraft:air", 0);
    // Independent fixture manifest:4x3x9 Sponge cells,12non-air cells and96explicit air.
    require(
        states.equals(states.stream().sorted().toList())
            && all == 108
            && air == 96
            && all - air == 12,
        "Sorted displayed exact states must account for96air +12nonair =108bounding cells");
    displayedTotals =
        Map.of(
            "items",
            itemTotals,
            "states",
            stateTotals,
            "allCells",
            all,
            "airCells",
            air,
            "nonAirCells",
            all - air);
  }

  private static void materialTotals() {
    @SuppressWarnings("unchecked")
    var lines = (List<String>) field(mc().gui.screen(), "lines");
    inspectMaterialRows(lines);
    String text = String.join("\n", lines);
    press("Copy all materials");
    String actual = mc().keyboardHandler.getClipboard();
    boolean matches = clipboardMatches(text, actual);
    // Counts/booleans only: an unrelated user clipboard must never enter QA logs/receipts.
    clipboardComparison =
        Map.of(
            "expectedLength",
            text.length(),
            "receivedLength",
            actual.length(),
            "rawEqual",
            actual.equals(text),
            "crlfPairs",
            actual.length() - actual.replace("\r\n", "\n").length(),
            "equalAfterCrlfNormalization",
            matches);
    require(matches, "Real Materials Copy widget clipboard: " + clipboardComparison);
    copied = true;
    try {
      var rows = SchematicMaterialScreen.class.getDeclaredMethod("rows");
      rows.setAccessible(true);
      int count = (int) rows.invoke(mc().gui.screen());
      materialPages =
          Math.max(1, (((List<?>) field(mc().gui.screen(), "wrapped")).size() + count - 1) / count);
      require(materialPages <= 16, "Unexpectedly many material pages");
    } catch (ReflectiveOperationException e) {
      throw new AssertionError(e);
    }
  }

  public static boolean clipboardMatches(String expected, String actual) {
    // Installed SDL's Windows backend expands LF to CRLF for CF_UNICODETEXT and retains it on read.
    // Preserve every row/character, lone CR and terminal newline; normalize only CRLF pairs.
    return actual.replace("\r\n", "\n").equals(expected);
  }

  /** Called from the existing tick-driven focused fixture, awaiting every async save callback. */
  public static boolean tick() {
    paused();
    if (complete) return true;
    if (captures.isEmpty()) {
      require(++toastWaitTicks <= 1200, "Vanilla toast queue did not clear before first frame");
      var manager = mc().gui.toastManager();
      if (!((Collection<?>) field(manager, "visibleToasts")).isEmpty()
          || !((Collection<?>) field(manager, "queued")).isEmpty()) {
        age = 0;
        return false;
      }
    }
    if (++age < 20) return false;
    if (pending != null) {
      if (!captureComplete) return false;
      saved();
    }
    if (!materials) {
      require(mc().gui.screen() instanceof RustyScreen, "Actual Build Planner settings screen");
      require(
          field(mc().gui.screen(), "selected").equals("build_planner"), "Selected Build Planner");
      if (!captured) {
        seen.addAll(labels());
        capture("build-planner-page-" + (plannerPage + 1));
        return false;
      }
      if (!seen.containsAll(REQUIRED)) {
        require(++plannerPage < 16, "Required real controls absent across settings pages: " + seen);
        press("More >");
        require(
            (int) field(mc().gui.screen(), "detailPage") == plannerPage,
            "Real detail-page navigation");
      } else {
        while ((int) field(mc().gui.screen(), "detailPage") > 0) press("<");
        for (int i = 0; !labels().contains("Materials / exact states"); i++) {
          require(i < 16, "Materials button must be reachable");
          press("More >");
        }
        press("Materials / exact states");
        require(
            mc().gui.screen() instanceof SchematicMaterialScreen,
            "Actual Materials widget opened screen");
        materials = true;
      }
    } else {
      require(
          mc().gui.screen() instanceof SchematicMaterialScreen,
          "Real material screen remains visible");
      if (!copied) materialTotals();
      if (!captured) {
        capture("materials-exact-states-page-" + (materialPage + 1));
        return false;
      }
      if (++materialPage < materialPages) {
        press(">");
        require(
            (int) field(mc().gui.screen(), "page") == materialPage,
            "Real Materials page navigation");
      } else {
        press("Done");
        require(
            mc().gui.screen() instanceof RustyScreen, "Materials Done returns to Build Planner");
        press("Done");
        require(mc().gui.screen() == null, "Settings Done returns to gameplay");
        complete = true;
        paused();
        return true;
      }
    }
    age = 0;
    captured = false;
    return false;
  }

  public static Map<String, Object> receipt() {
    return Map.of(
        "complete",
        complete,
        "requiredControls",
        REQUIRED,
        "observedControls",
        seen,
        "materialsCopyThroughWidget",
        copied,
        "clipboardComparison",
        clipboardComparison,
        "displayedTotals",
        displayedTotals,
        "toastWaitTicks",
        toastWaitTicks,
        "captures",
        captures);
  }
}
