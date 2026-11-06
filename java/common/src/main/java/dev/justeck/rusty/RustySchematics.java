package dev.justeck.rusty;

import dev.justeck.rusty.core.Schematic;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.Mirror;
import net.minecraft.world.level.block.Rotation;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraft.world.phys.BlockHitResult;

/** Transient world-scoped schematic session. Palette validation/preview work is capped per tick. */
public final class RustySchematics {
  private static Object level, connection;
  private static String scope = "";
  public static Schematic.Document document;
  private static Schematic.Document pending;
  private static Iterator<Schematic.State> validation;
  private static Map<Schematic.State, BlockState> resolved = new HashMap<>(),
      validating = new HashMap<>();
  private static final Map<BlockPos, BlockState> targets = new LinkedHashMap<>();
  private static Iterator<Schematic.Cell> rebuilding;
  private static final List<Map.Entry<BlockPos, BlockState>> previewTargets = new ArrayList<>();
  private static BlockPos origin, first, second;
  private static int turns, previewCursor;
  private static boolean mirror, transformFailed;
  public static String result = "No schematic loaded";
  public static int validationWork, transformWork, previewWork;

  public static boolean loaded() {
    return document != null;
  }

  public static boolean ready() {
    return loaded() && rebuilding == null && pending == null && origin != null && !transformFailed;
  }

  public static Map<BlockPos, BlockState> targets() {
    return Collections.unmodifiableMap(targets);
  }

  public static BlockPos originPos() {
    return origin;
  }

  public static void scopeTick() {
    var mc = Minecraft.getInstance();
    if (level != mc.level
        || connection != mc.getConnection()
        || !scope.equals(RustyClient.scope())) {
      SchematicConstruction.stop("World or connection changed");
      SchematicRotation.reset();
      SchematicFiles.cancel();
      document = null;
      pending = null;
      validation = null;
      rebuilding = null;
      transformFailed = false;
      previewTargets.clear();
      targets.clear();
      resolved.clear();
      validating.clear();
      origin = first = second = null;
      turns = 0;
      mirror = false;
      level = mc.level;
      connection = mc.getConnection();
      scope = RustyClient.scope();
      result = "No schematic loaded";
    }
  }

  public static void clear() {
    SchematicConstruction.stop("Schematic cleared");
    SchematicFiles.cancel();
    document = null;
    pending = null;
    validation = null;
    rebuilding = null;
    transformFailed = false;
    previewTargets.clear();
    RustyPlanner.cells = List.of();
    targets.clear();
    resolved.clear();
    validating.clear();
    result = "Schematic cleared; box planner available";
  }

  static void cancelImport() {
    if (pending == null) return;
    pending = null;
    validation = null;
    validating.clear();
    result =
        document == null
            ? "Import cancelled; no schematic loaded"
            : "Import cancelled; previous schematic retained";
  }

  public static void accept(Schematic.Document next) {
    SchematicConstruction.stop("New schematic selected");
    pending = next;
    validation = next.palette().iterator();
    validating.clear();
    result = "Validating current-game block states";
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static BlockState resolve(Schematic.State state) {
    var id = Identifier.parse(state.id());
    if (!BuiltInRegistries.BLOCK.containsKey(id))
      throw new IllegalArgumentException("Unknown block " + id);
    var block = BuiltInRegistries.BLOCK.getValue(id);
    var out = block.defaultBlockState();
    for (var entry : state.properties().entrySet()) {
      Property property = block.getStateDefinition().getProperty(entry.getKey());
      if (property == null)
        throw new IllegalArgumentException("Unknown property " + entry.getKey() + " on " + id);
      var value = property.getValue(entry.getValue());
      if (value.isEmpty()) throw new IllegalArgumentException("Invalid property value on " + id);
      out = out.setValue(property, (Comparable) value.get());
    }
    return out;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static Schematic.State describe(BlockState state) {
    var p = new TreeMap<String, String>();
    for (var property : state.getProperties()) {
      Property raw = property;
      p.put(property.getName(), raw.getName((Comparable) state.getValue(property)));
    }
    return new Schematic.State(BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString(), p);
  }

  public static void origin() {
    var mc = Minecraft.getInstance();
    if (mc.hitResult instanceof BlockHitResult hit) {
      SchematicConstruction.stop("Origin changed");
      origin = hit.getBlockPos().relative(hit.getDirection());
      rebuild();
      RustyClient.notice = "Schematic origin set at crosshair";
    }
  }

  public static void rotate() {
    SchematicConstruction.stop("Rotation changed");
    turns = (turns + 1) % 4;
    rebuild();
  }

  public static void mirror() {
    SchematicConstruction.stop("Mirror changed");
    mirror = !mirror;
    rebuild();
  }

  private static void rebuild() {
    transformFailed = false;
    rebuilding = null;
    RustyPlanner.cells = List.of();
    targets.clear();
    previewTargets.clear();
    previewCursor = 0;
    if (document != null && origin != null) rebuilding = document.cells().iterator();
    result = "Preparing transformed preview";
  }

  public static String materialText() {
    return document == null ? "No schematic loaded" : SchematicMaterials.text(document, resolved);
  }

  public static void copyMaterials() {
    Minecraft.getInstance().keyboardHandler.setClipboard(materialText());
    RustyClient.notice = "Copied material and exact state totals";
  }

  public static void corner(boolean last) {
    var mc = Minecraft.getInstance();
    if (mc.hitResult instanceof BlockHitResult hit
        && SchematicConstruction.received(hit.getBlockPos())) {
      if (last) second = hit.getBlockPos().immutable();
      else first = hit.getBlockPos().immutable();
      RustyClient.notice = "Export " + (last ? "second" : "first") + " corner set (states only)";
    } else RustyClient.notice = "Aim at a settled received block first";
  }

  public static BlockPos first() {
    return first;
  }

  public static BlockPos second() {
    return second;
  }

  static void advanceValidation() {
    if (pending != null)
      try {
        while (validation.hasNext() && validationWork < 512) {
          validationWork++;
          var s = validation.next();
          validating.put(s, resolve(s));
        }
        if (!validation.hasNext()) {
          document = pending;
          pending = null;
          resolved = validating;
          validating = new HashMap<>();
          turns = 0;
          mirror = false;
          if (origin == null && Minecraft.getInstance().player != null)
            origin = Minecraft.getInstance().player.blockPosition().offset(2, 0, 0);
          rebuild();
          RustyClient.notice =
              "Loaded "
                  + document.format()
                  + "; "
                  + document.detailCount()
                  + " data details require manual restoration";
        }
      } catch (RuntimeException e) {
        pending = null;
        validation = null;
        validating.clear();
        RustyClient.notice = "Schematic rejected: " + e.getMessage();
        result = RustyClient.notice;
      }
  }

  static void advanceTransform() {
    if (rebuilding != null)
      try {
        while (rebuilding.hasNext() && transformWork < 2048) {
          transformWork++;
          var c = rebuilding.next();
          var p = Schematic.transform(c.pos(), turns, mirror);
          var at =
              new BlockPos(
                  Math.addExact(origin.getX(), p.x()),
                  Math.addExact(origin.getY(), p.y()),
                  Math.addExact(origin.getZ(), p.z()));
          var state = resolved.get(c.state());
          if (!c.state().skipped()) {
            state =
                state
                    .mirror(mirror ? Mirror.FRONT_BACK : Mirror.NONE)
                    .rotate(Rotation.values()[turns]);
            targets.put(at, state);
            previewTargets.add(Map.entry(at, state));
          }
        }
        if (!rebuilding.hasNext()) {
          rebuilding = null;
          result = targets.size() + " planned cells; explicit Start required";
        }
      } catch (RuntimeException e) {
        rebuilding = null;
        transformFailed = true;
        targets.clear();
        previewTargets.clear();
        RustyPlanner.cells = List.of();
        result = "Transform exceeds coordinate limits; cannot start construction";
        RustyClient.notice = result;
        SchematicConstruction.result = result;
      }
  }

  public static void tick(boolean focused) {
    validationWork = transformWork = previewWork = 0;
    advanceValidation();
    advanceTransform();
    if (ready() && RustyClient.active("build_planner")) {
      var entries =
          previewTargets.subList(
              previewCursor, Math.min(previewTargets.size(), previewCursor + 128));
      var cells = new ArrayList<RustyPlanner.Cell>();
      for (var e : entries) {
        previewWork++;
        int color =
            !SchematicConstruction.received(e.getKey())
                ? 0xff777777
                : Minecraft.getInstance().level.getBlockState(e.getKey()).equals(e.getValue())
                    ? 0xff68b879
                    : Minecraft.getInstance().level.getBlockState(e.getKey()).isAir()
                        ? 0xffd6a058
                        : 0xffcc6262;
        cells.add(new RustyPlanner.Cell(e.getKey(), color));
      }
      RustyPlanner.cells = List.copyOf(cells);
      previewCursor += 128;
      if (previewCursor >= targets.size()) previewCursor = 0;
    }
    SchematicFiles.tick();
    SchematicConstruction.tick(focused);
  }
}
