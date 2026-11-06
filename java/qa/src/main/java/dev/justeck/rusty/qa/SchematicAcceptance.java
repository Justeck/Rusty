package dev.justeck.rusty.qa;

import dev.justeck.rusty.*;
import dev.justeck.rusty.core.*;
import java.nio.file.*;
import java.util.*;
import java.util.function.BooleanSupplier;
import net.minecraft.client.Minecraft;
import net.minecraft.core.*;
import net.minecraft.world.item.*;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.phys.*;

/** Disposable RustyLab fixture, selected only by RUSTY_QA_SCHEMATICS=1 and existing QA loader. */
public final class SchematicAcceptance {
  public static volatile boolean rejectNextSwap;
  public static volatile int rejectedInventory, rejectedPlacements;
  public static volatile boolean rejectNextPlacement;
  private static List<ItemStack> permissionInventory;
  private static Vec3 permissionPosition;
  private static int permissionUses,
      permissionPlaced,
      permissionSourced,
      permissionWalked,
      permissionSlot;
  private static boolean awaitingRevocation, receivedRevocation;
  private static Map<BlockPos, net.minecraft.world.level.block.state.BlockState> permissionWorld;

  private static void permissionBaseline() {
    permissionInventory = new ArrayList<>();
    for (int i = 0; i < 36; i++)
      permissionInventory.add(mc().player.getInventory().getItem(i).copy());
    permissionPosition = mc().player.position();
    permissionSlot = mc().player.getInventory().getSelectedSlot();
    permissionUses = SchematicLookEvidence.uses;
    permissionPlaced = SchematicConstruction.placed;
    permissionSourced = SchematicConstruction.sourced;
    permissionWalked = SchematicConstruction.walked;
    var cells = new HashMap<BlockPos, net.minecraft.world.level.block.state.BlockState>();
    for (var at : RustySchematics.targets().keySet()) cells.put(at, mc().level.getBlockState(at));
    permissionWorld = Map.copyOf(cells);
  }

  public static void permissionReceipt() {
    if (awaitingRevocation && !SchematicConstruction.buildEligible()) {
      expect(SchematicConstruction.active(), "Revocation must reach an actual active run");
      permissionBaseline();
      receivedRevocation = true;
      awaitingRevocation = false;
    }
  }

  private static boolean permissionUnchanged() {
    expect(!SchematicConstruction.active(), "Permission refusal must relinquish control");
    expect(
        SchematicLookEvidence.uses == permissionUses
            && SchematicConstruction.placed == permissionPlaced
            && SchematicConstruction.sourced == permissionSourced
            && SchematicConstruction.walked == permissionWalked,
        "No walking, sourcing, or use dispatch after permission boundary");
    expect(
        mc().player.position().distanceToSqr(permissionPosition) < .01,
        "No automatic displacement");
    for (int i = 0; i < 36; i++)
      expect(
          ItemStack.matches(permissionInventory.get(i), mc().player.getInventory().getItem(i)),
          "Inventory remains exact");
    for (var e : permissionWorld.entrySet())
      expect(mc().level.getBlockState(e.getKey()).equals(e.getValue()), "World remains exact");
    return true;
  }

  private record Step(String name, Runnable action, BooleanSupplier condition, int minimum) {}

  private static final List<Step> steps = new ArrayList<>();
  private static final List<Map<String, Object>> checks = new ArrayList<>();
  private static final List<Map<String, Object>> outcomeEvidence = new ArrayList<>();
  private static boolean prepared, finished;
  private static int index, wait, ticks;
  private static Path fixture, export;
  private static String oldClipboard;
  private static ItemStack displaced;
  private static int completedPlacements;
  private static Schematic.Document retainedDocument;

  private static void add(String n, Runnable a, BooleanSupplier c, int min) {
    steps.add(new Step(n, a, c, min));
  }

  private static Minecraft mc() {
    return Minecraft.getInstance();
  }

  public static String damageCommand() {
    return "damage @a[limit=1] 1 minecraft:generic";
  }

  private static void command(String text) {
    var server = mc().getSingleplayerServer();
    if (server == null)
      throw new IllegalStateException("Fixture commands require owned integrated server");
    server.execute(() ->
        server
            .getCommands()
            .performPrefixedCommand(
                server.createCommandSourceStack().withSuppressedOutput(), text));
  }

  private static void expect(boolean value, String reason) {
    if (!value) throw new AssertionError(reason + "; " + SchematicConstruction.status());
  }

  private static boolean stopped(String text) {
    return !SchematicConstruction.active()
        && SchematicConstruction.result
            .toLowerCase(Locale.ROOT)
            .contains(text.toLowerCase(Locale.ROOT));
  }

  private static void origin(int x, int y, int z) {
    mc().hitResult =
        new BlockHitResult(
            new Vec3(x + .5, y - .0, z + .5), Direction.UP, new BlockPos(x, y - 1, z), false);
    RustySchematics.origin();
  }

  private static void clearControllers() {
    SchematicConstruction.stop("Fixture reset");
    RustyMining.stop("Fixture reset");
    RustyNavigation.cancelWalk();
    mc().gui.setScreen(null);
    mc().options.keyUp.setDown(false);
    mc().options.keyAttack.setDown(false);
    mc().options.keyUse.setDown(false);
  }

  public static void tick() {
    if (finished) return;
    ticks++;
    try {
      String directory =
          mc().gameDirectory.toPath().toAbsolutePath().normalize().toString().replace('\\', '/');
      if (!directory.endsWith("/java/fabric/run") && !directory.endsWith("/java/neoforge/run"))
        throw new IllegalStateException("Refusing non-development directory");
      if (mc().player == null || mc().level == null) {
        if (ticks > 2400) throw new AssertionError("Timed out joining RustyLab");
        return;
      }
      if (!prepared) {
        if (mc().hasSingleplayerServer()) {
          var world =
              mc().getSingleplayerServer()
                  .getWorldPath(net.minecraft.world.level.storage.LevelResource.ROOT)
                  .toRealPath();
          if (!world.equals(mc().gameDirectory.toPath().resolve("saves/RustyLab").toRealPath()))
            throw new IllegalStateException("Refusing personal world");
          prepare();
        } else {
          if (mc().getCurrentServer() == null
              || !mc().getCurrentServer().ip.startsWith("127.0.0.1:"))
            throw new IllegalStateException("Refusing non-loopback server");
          prepareDenied();
        }
        prepared = true;
      }
      if (index >= steps.size()) {
        finish(null);
        return;
      }
      var step = steps.get(index);
      if (wait == 0) {
        RustyClient.LOG.info("Schematic fixture {}: {}", index, step.name);
        step.action.run();
      }
      wait++;
      if (wait >= step.minimum && step.condition.getAsBoolean()) {
        checks.add(Map.of("name", step.name, "passed", true));
        index++;
        wait = 0;
        write(false, null);
      } else if (wait > 3600)
        throw new AssertionError("Timed out: " + step.name + "; " + SchematicConstruction.status());
    } catch (Throwable e) {
      RustyClient.LOG.error("Schematic game acceptance failed", e);
      finish(e.toString());
    }
  }

  private static void prepare() throws Exception {
    String root = System.getenv("RUSTY_QA_SCHEMATIC_FIXTURES");
    if (root == null)
      throw new IllegalStateException(
          "Set RUSTY_QA_SCHEMATIC_FIXTURES to independent fixture directory");
    fixture = Path.of(root).toRealPath();
    oldClipboard = mc().keyboardHandler.getClipboard();
    export =
        mc().gameDirectory
            .toPath()
            .resolve("rusty-qa-states-" + System.currentTimeMillis() + ".schem");
    mc().getTutorial().setStep(net.minecraft.client.tutorial.TutorialSteps.NONE);
    RustyClient.config = new Settings();
    RustyClient.set("build_planner", true);
    add(
        "Owned received platform, survival, full hotbar and main-only materials",
        () -> {
          clearControllers();
          command("fill -18 199 -18 18 204 22 air");
          command("fill -18 198 -18 18 199 22 stone");
          command("setblock 3 202 11 stone");
          command("tp @a 0.5 200 -4.5 0 20");
          command("gamemode survival @a");
          command("clear @a");
          command("effect give @a instant_health 1 10");
          command("effect give @a saturation 1 10");
          String[] tools = {
            "iron_pickaxe",
            "iron_axe",
            "iron_shovel",
            "iron_hoe",
            "wooden_pickaxe",
            "wooden_axe",
            "wooden_shovel",
            "wooden_hoe",
            "stone_pickaxe"
          };
          for (int i = 0; i < tools.length; i++)
            command("item replace entity @a hotbar." + i + " with " + tools[i]);
          String[] items = {
            "cobblestone 2",
            "oak_planks 2",
            "glass 1",
            "oak_log 1",
            "oak_slab 1",
            "oak_stairs 1",
            "oak_door 1",
            "red_bed 1"
          };
          for (int i = 0; i < items.length; i++)
            command("item replace entity @a inventory." + i + " with " + items[i]);
        },
        () -> {
          if (mc().player.getY() < 199
              || !mc().player
                  .getInventory()
                  .getItem(16)
                  .is(
                      net.minecraft.core.registries.BuiltInRegistries.ITEM.getValue(
                          net.minecraft.resources.Identifier.parse("minecraft:red_bed"))))
            return false;
          for (int i = 0; i < 9; i++)
            expect(
                !mc().player.getInventory().getItem(i).isEmpty()
                    && !(mc().player.getInventory().getItem(i).getItem() instanceof BlockItem),
                "Hotbar must be full of non-block items");
          displaced = mc().player.getInventory().getItem(0).copy();
          return true;
        },
        40);
    for (String extension : new String[] {"litematic", "nbt", "schem"})
      add(
          "Actual bounded path import of independent " + extension,
          () -> SchematicFiles.loadPath(fixture.resolve("multi-material." + extension)),
          () -> {
            if (SchematicFiles.busy || !RustySchematics.ready()) return false;
            expect(RustySchematics.targets().size() == 12, "Expected 12 named-state cells");
            expect(!SchematicConstruction.active(), "Import never starts construction");
            return true;
          },
          10);
    add(
        "Origin and transformed native state preview remain paused",
        () -> {
          origin(0, 200, 3);
          RustySchematics.rotate();
          RustySchematics.mirror();
        },
        () -> {
          if (!RustySchematics.ready()) return false;
          expect(!SchematicConstruction.active(), "Transform never starts construction");
          expect(
              RustySchematics.transformWork <= 2049 && RustySchematics.previewWork <= 128,
              "Per-tick transform and preview budget");
          RustySchematics.mirror();
          for (int i = 0; i < 3; i++) RustySchematics.rotate();
          return true;
        },
        5);
    add(
        "Copy uses actual clipboard with sorted registry items and exact coupled quantities",
        RustySchematics::copyMaterials,
        () -> {
          String text = mc().keyboardHandler.getClipboard();
          for (String item :
              new String[] {
                "minecraft:glass]: 1",
                "minecraft:oak_door]: 1",
                "minecraft:red_bed]: 1",
                "minecraft:cobblestone]: 2",
                "minecraft:oak_planks]: 2"
              }) expect(text.contains(item), "Material total " + item);
          expect(text.contains("Exact block-state totals"), "Every state remains visible");
          expect(!SchematicConstruction.active(), "Copy never starts construction");
          return true;
        },
        5);
    add(
        "Real Build Planner and Materials widgets capture paused controls and exact totals",
        SchematicUiEvidence::begin,
        SchematicUiEvidence::tick,
        1);
    add(
        "Explicit construction starts beyond normal reach",
        () -> {
          expect(RustySchematics.ready(), "Ready schematic");
          mc().getConnection()
              .send(
                  new net.minecraft.network.protocol.game.ServerboundClientCommandPacket(
                      net.minecraft.network.protocol.game.ServerboundClientCommandPacket.Action
                          .REQUEST_STATS));
          SchematicLookEvidence.resetUses();
          expect(SchematicConstruction.start(), "Explicit start succeeds");
        },
        SchematicConstruction::active,
        1);
    add(
        "Received multi-material construction walks, sources and places exact directional/coupled"
            + " states",
        () -> {},
        () -> {
          if (SchematicConstruction.active()) return false;
          expect(stopped("Complete"), "Must finish with exact received verification");
          expect(SchematicConstruction.walked > 0, "Actual ordinary walking required");
          expect(
              SchematicConstruction.sourced >= 6,
              "Multiple main-inventory source acknowledgments required");
          expect(
              SchematicLookEvidence.uses == 10 && SchematicLookEvidence.violations == 0,
              "Every actual construction use is preceded by a matching ordinary rotation");
          expect(
              SchematicConstruction.placed == 10,
              "Ten real placement item actions including coupled writes");
          for (var e : RustySchematics.targets().entrySet())
            expect(
                SchematicConstruction.received(e.getKey())
                    && mc().level.getBlockState(e.getKey()).equals(e.getValue()),
                "Exact settled state " + e.getKey());
          boolean preserved = false;
          for (int i = 0; i < 36; i++)
            preserved |= ItemStack.matches(displaced, mc().player.getInventory().getItem(i));
          expect(preserved, "Original displaced stack preserved exactly");
          completedPlacements = SchematicConstruction.placed;
          return true;
        },
        20);
    add(
        "Two real crosshair corners capture received states and atomic export",
        () -> {
          mc().hitResult =
              new BlockHitResult(
                  Vec3.atCenterOf(new BlockPos(0, 200, 3)),
                  Direction.UP,
                  new BlockPos(0, 200, 3),
                  false);
          RustySchematics.corner(false);
          mc().hitResult =
              new BlockHitResult(
                  Vec3.atCenterOf(new BlockPos(3, 202, 11)),
                  Direction.UP,
                  new BlockPos(3, 202, 11),
                  false);
          RustySchematics.corner(true);
          SchematicFiles.savePath(export);
        },
        () -> Files.isRegularFile(export) && !SchematicFiles.busy,
        10);
    add(
        "Independent roundtrip verifies states-only export rather than predicted/unknown air",
        () -> {},
        () -> {
          try {
            var exported = Schematic.read(Files.readAllBytes(export));
            expect(
                exported.cells().size() == 108 && exported.detailCount() == 0,
                "Dense states-only108cell export");
            for (var cell : exported.cells())
              expect(
                  RustySchematics.resolve(cell.state())
                      .equals(
                          mc().level
                              .getBlockState(
                                  new BlockPos(
                                      cell.pos().x(), 200 + cell.pos().y(), 3 + cell.pos().z()))),
                  "Export actual received state");
            return true;
          } catch (Exception e) {
            throw new RuntimeException(e);
          }
        },
        2);
    add(
        "Existing destination preserved without overwrite confirmation",
        () -> SchematicFiles.savePath(export),
        () -> !SchematicFiles.busy && RustyClient.notice.contains("preserved"),
        5);
    add(
        "Cancelled/stale asynchronous result never replaces current document",
        () -> {
          var before = RustySchematics.document;
          SchematicFiles.loadPath(fixture.resolve("multi-material.nbt"));
          SchematicFiles.cancel();
          expect(RustySchematics.document == before, "Transactional cancellation");
        },
        () -> !SchematicFiles.busy && !SchematicConstruction.active(),
        10);
    add(
        "Cancel import after an actual512-state registry-validation batch",
        () -> {
          retainedDocument = RustySchematics.document;
          var palette =
              net.minecraft.core.registries.BuiltInRegistries.BLOCK.stream()
                  .flatMap(block -> block.getStateDefinition().getPossibleStates().stream())
                  .limit(1200)
                  .map(RustySchematics::describe)
                  .toList();
          expect(palette.size() > 512, "Multi-frame registry validation fixture");
          RustySchematics.accept(
              new Schematic.Document(
                  retainedDocument.cells(), 0, "QA cancellation during validation", palette));
        },
        () -> {
          if (RustySchematics.validationWork != 512
              || RustySchematics.document != retainedDocument
              || RustySchematics.ready()) return false;
          SchematicFiles.cancel();
          return true;
        },
        1);
    add(
        "Cancelled validation cannot later activate after multiple remaining batches",
        () -> {},
        () -> {
          expect(
              RustySchematics.document == retainedDocument && RustySchematics.ready(),
              "Previous accepted document retained after cancellation window");
          return true;
        },
        10);
    add(
        "Overflowed transform cannot start or claim a new complete plan",
        () -> {
          origin(1, 200, 0);
          RustySchematics.accept(
              new Schematic.Document(
                  List.of(
                      new Schematic.Cell(
                          new Schematic.Pos(Integer.MAX_VALUE, 0, 0),
                          Schematic.State.parse("minecraft:stone"))),
                  0,
                  "QA overflow"));
        },
        () -> {
          if (!RustySchematics.result.contains("Transform exceeds")) return false;
          expect(
              !RustySchematics.ready()
                  && RustySchematics.targets().isEmpty()
                  && RustyPlanner.cells.isEmpty(),
              "Failed transform is nonready with empty preview");
          expect(!SchematicConstruction.start(), "Overflow plan cannot start");
          expect(
              !SchematicConstruction.active()
                  && !SchematicConstruction.result.startsWith("Complete"),
              "No false Complete after overflow");
          return true;
        },
        5);
    add(
        "Restore accepted document after transform-error check",
        () -> {
          origin(0, 200, 3);
          RustySchematics.accept(retainedDocument);
        },
        RustySchematics::ready,
        5);
    add(
        "Unused unknown declared palette entry rejects before activation",
        () -> {
          try {
            var nbtRoot = new net.minecraft.nbt.CompoundTag();
            var schematic = new net.minecraft.nbt.CompoundTag();
            schematic.putInt("Version", 3);
            schematic.putInt(
                "DataVersion",
                net.minecraft.SharedConstants.getCurrentVersion().dataVersion().version());
            schematic.putShort("Width", (short) 1);
            schematic.putShort("Height", (short) 1);
            schematic.putShort("Length", (short) 1);
            var blocks = new net.minecraft.nbt.CompoundTag();
            var palette = new net.minecraft.nbt.CompoundTag();
            palette.putInt("minecraft:stone", 0);
            palette.putInt("unknown:unused_declared", 1);
            blocks.put("Palette", palette);
            blocks.putByteArray("Data", new byte[] {0});
            schematic.put("Blocks", blocks);
            nbtRoot.put("Schematic", schematic);
            var path =
                mc().gameDirectory
                    .toPath()
                    .resolve("rusty-qa-unused-palette-" + System.nanoTime() + ".schem");
            net.minecraft.nbt.NbtIo.writeCompressed(nbtRoot, path);
            SchematicFiles.loadPath(path);
          } catch (Exception e) {
            throw new RuntimeException(e);
          }
        },
        () -> {
          if (SchematicFiles.busy || !RustySchematics.ready()) return false;
          expect(
              RustySchematics.document == retainedDocument
                  && RustyClient.notice.contains("Unknown block"),
              "Unused declared unknown must reject and retain accepted document");
          return true;
        },
        10);
    add(
        "Planned existing SINGLE chest is protected from a new RIGHT chest neighbor",
        () -> {
          command("setblock -8 200 4 chest[facing=north,type=single]");
          command("setblock -7 200 4 air");
          command("item replace entity @a inventory.8 with chest");
          command("tp @a -7.5 200 0.5 0 20");
          origin(0, 200, 0);
          var single =
              net.minecraft.core.registries.BuiltInRegistries.BLOCK
                  .getValue(net.minecraft.resources.Identifier.parse("minecraft:chest"))
                  .defaultBlockState()
                  .setValue(net.minecraft.world.level.block.ChestBlock.FACING, Direction.NORTH)
                  .setValue(
                      net.minecraft.world.level.block.ChestBlock.TYPE,
                      net.minecraft.world.level.block.state.properties.ChestType.SINGLE);
          RustySchematics.accept(
              new Schematic.Document(
                  List.of(
                      new Schematic.Cell(
                          new Schematic.Pos(-8, 0, 4), RustySchematics.describe(single)),
                      new Schematic.Cell(
                          new Schematic.Pos(-7, 0, 4),
                          RustySchematics.describe(
                              single.setValue(
                                  net.minecraft.world.level.block.ChestBlock.TYPE,
                                  net.minecraft.world.level.block.state.properties.ChestType
                                      .RIGHT)))),
                  0,
                  "QA planned reactive chest"));
        },
        () -> {
          if (!RustySchematics.ready() || !mc().player.getInventory().getItem(17).is(Items.CHEST))
            return false;
          var at = new BlockPos(-7, 200, 4);
          var before = mc().level.getBlockState(new BlockPos(-8, 200, 4));
          expect(
              before.getValue(net.minecraft.world.level.block.ChestBlock.TYPE)
                  == net.minecraft.world.level.block.state.properties.ChestType.SINGLE,
              "Existing received chest stays SINGLE");
          expect(
              SchematicPlacement.find(
                      at, RustySchematics.targets().get(at), mc().player.getInventory().getItem(17))
                  == null,
              "Unsafe synchronous planned-neighbor mutation refused before dispatch");
          expect(mc().level.getBlockState(at).isAir(), "No reactive neighbor placement occurred");
          return true;
        },
        20);
    add(
        "Original box planner is still available after clear",
        () -> {
          RustySchematics.clear();
          mc().hitResult =
              new BlockHitResult(
                  new Vec3(-4.5, 200, -3.5), Direction.UP, new BlockPos(-5, 199, -4), false);
          RustyPlanner.anchor();
        },
        () -> !RustySchematics.loaded() && !RustyPlanner.cells.isEmpty(),
        5);
    addPartialAndStops();
  }

  private static void oneCell(
      BlockPos target, net.minecraft.world.level.block.state.BlockState state) {
    origin(0, 200, 0);
    RustySchematics.accept(
        new Schematic.Document(
            List.of(
                new Schematic.Cell(
                    new Schematic.Pos(target.getX(), target.getY() - 200, target.getZ()),
                    RustySchematics.describe(state))),
            0,
            "Owned QA one-cell state"));
  }

  private static void addPartialAndStops() {
    add(
        "Partial slab and snow use repeated ordinary item actions",
        () -> {
          clearControllers();
          command("tp @a 7.5 200 -3.5 0 20");
          command("item replace entity @a inventory.0 with oak_slab 2");
          command("item replace entity @a inventory.1 with snow 4");
          origin(0, 200, 0);
          RustySchematics.accept(
              new Schematic.Document(
                  List.of(
                      new Schematic.Cell(
                          new Schematic.Pos(8, 0, 0),
                          RustySchematics.describe(
                              Blocks.OAK_SLAB
                                  .defaultBlockState()
                                  .setValue(
                                      BlockStateProperties.SLAB_TYPE,
                                      net.minecraft.world.level.block.state.properties.SlabType
                                          .DOUBLE))),
                      new Schematic.Cell(
                          new Schematic.Pos(9, 0, 0),
                          RustySchematics.describe(
                              Blocks.SNOW
                                  .defaultBlockState()
                                  .setValue(BlockStateProperties.LAYERS, 4)))),
                  0,
                  "Owned partial fixture"));
        },
        RustySchematics::ready,
        20);
    add(
        "Partial-material copy counts two slabs and four snow items",
        () -> {
          RustySchematics.copyMaterials();
          expect(
              mc().keyboardHandler.getClipboard().contains("minecraft:oak_slab]: 2")
                  && mc().keyboardHandler.getClipboard().contains("minecraft:snow]: 4"),
              "Partial material quantities");
          expect(SchematicConstruction.start(), "Partial build starts");
        },
        SchematicConstruction::active,
        1);
    add(
        "Partial-material exact received completion",
        () -> {},
        () -> {
          if (SchematicConstruction.active()) return false;
          expect(stopped("Complete"), "Partial build complete");
          expect(
              SchematicConstruction.placed == 6, "Exactly six acknowledged accumulation actions");
          return true;
        },
        10);
    add(
        "Obstructed target is preserved without demolition",
        () -> {
          oneCell(new BlockPos(11, 200, 0), Blocks.COBBLESTONE.defaultBlockState());
          command("setblock 11 200 0 bedrock");
        },
        RustySchematics::ready,
        10);
    add(
        "Obstruction yields explicit manual result",
        () -> expect(SchematicConstruction.start(), "Obstruction survey starts"),
        () -> {
          if (SchematicConstruction.active()) return false;
          expect(
              mc().level.getBlockState(new BlockPos(11, 200, 0)).is(Blocks.BEDROCK),
              "Obstruction preserved");
          expect(stopped("manual"), "Manual leftover reported");
          return true;
        },
        5);
    add(
        "Missing material setup",
        () -> {
          command("setblock 11 200 0 air");
          command("clear @a cobblestone");
        },
        () -> mc().level.getBlockState(new BlockPos(11, 200, 0)).isAir(),
        10);
    add(
        "Missing material stops without phantom completion",
        () -> expect(SchematicConstruction.start(), "Missing-material run starts"),
        () -> stopped("Missing material") && SchematicConstruction.placed == 0,
        5);
    add(
        "Owned server inventory-rejection setup keeps material only in main inventory",
        () -> {
          command("clear @a cobblestone");
          command("item replace entity @a hotbar.0 with iron_pickaxe");
          command("item replace entity @a inventory.0 with cobblestone 8");
          command("tp @a 10.5 200 -2.5 0 20");
        },
        () ->
            mc().player.getInventory().getItem(9).is(Items.COBBLESTONE)
                && mc().player.getInventory().getItem(0).is(Items.IRON_PICKAXE),
        20);
    add(
        "Real rejected server swap cannot pass an older stats response or trigger placement",
        () -> {
          rejectNextSwap = true;
          mc().getConnection()
              .send(
                  new net.minecraft.network.protocol.game.ServerboundClientCommandPacket(
                      net.minecraft.network.protocol.game.ServerboundClientCommandPacket.Action
                          .REQUEST_STATS));
          expect(SchematicConstruction.start(), "Inventory rejection run starts");
        },
        () -> {
          if (SchematicConstruction.active()) return false;
          expect(rejectedInventory == 1, "Owned server rejected one actual inventory packet");
          expect(
              SchematicConstruction.sourced == 0 && SchematicConstruction.placed == 0,
              "No rejected swap or placement counted");
          expect(stopped("inventory"), "Inventory rejection reported");
          expect(
              mc().level.getBlockState(new BlockPos(11, 200, 0)).isAir(),
              "No target placement before processing confirmation");
          expect(
              mc().player.getInventory().getItem(9).is(Items.COBBLESTONE)
                  && mc().player.getInventory().getItem(0).is(Items.IRON_PICKAXE),
              "Authoritative correction retained both original stacks");
          return true;
        },
        10);
    add(
        "Denied Start setup receives real Adventure permission",
        () -> {
          command("gamemode adventure @a");
          command("item replace entity @a inventory.2 with cobblestone 8");
          command("tp @a 10.5 200 -2.5 0 20");
        },
        () ->
            !SchematicConstruction.buildEligible()
                && mc().player.getInventory().getItem(11).is(Items.COBBLESTONE),
        20);
    add(
        "Denied Start changes no world, inventory, selection, walking or use",
        () -> {
          permissionBaseline();
          expect(
              !SchematicConstruction.start(), "Adventure refuses Start before automatic control");
          expect(
              RustyClient.notice.contains("Survival or Creative build permission"),
              "Truthful permission refusal");
        },
        () -> {
          expect(
              mc().player.getInventory().getSelectedSlot() == permissionSlot,
              "Denied Start keeps selection");
          return permissionUnchanged();
        },
        20);
    add(
        "Restore Survival for genuine ordinary placement rejection",
        () -> {
          command("gamemode survival @a");
          command("item replace entity @a hotbar.0 with cobblestone 8");
        },
        () ->
            SchematicConstruction.buildEligible()
                && mc().player.getInventory().getItem(0).is(Items.COBBLESTONE),
        20);
    add(
        "Actual ordinary use is refused by owned server and never counted",
        () -> {
          rejectNextPlacement = true;
          expect(SchematicConstruction.start(), "Authorized rejection fixture starts");
        },
        () -> {
          if (SchematicConstruction.active()) return false;
          expect(rejectedPlacements == 1, "Server refused one actually dispatched ordinary use");
          expect(
              SchematicConstruction.placed == 0 && stopped("rejected"),
              "Refusal has no completed action");
          expect(
              mc().level.getBlockState(new BlockPos(11, 200, 0)).isAir(),
              "Ordinary server correction retained air");
          return true;
        },
        10);
    add(
        "Active permission revocation baseline uses multi-batch read-only survey",
        () -> {
          var cells = new ArrayList<Schematic.Cell>();
          for (int y = 0; y < 32; y++)
            for (int z = 0; z < 16; z++)
              for (int x = 0; x < 16; x++)
                cells.add(
                    new Schematic.Cell(
                        new Schematic.Pos(x, y, z), Schematic.State.parse("minecraft:stone")));
          RustySchematics.accept(new Schematic.Document(cells, 0, "Owned permission survey"));
          origin(0, 220, 0);
        },
        RustySchematics::ready,
        10);
    add(
        "Actual mode revocation stops before next automatic action",
        () -> {
          expect(SchematicConstruction.start(), "Revocation starts an eligible run");
          awaitingRevocation = true;
          receivedRevocation = false;
          command("gamemode adventure @a");
        },
        () -> receivedRevocation && stopped("permission") && permissionUnchanged(),
        3);
    add(
        "Restoring permission never rearms stopped construction",
        () -> command("gamemode survival @a"),
        () -> SchematicConstruction.buildEligible() && permissionUnchanged(),
        20);
    add(
        "Restore owned survival for cancellation tests",
        () -> {
          command("gamemode survival @a");
          command("tp @a 7.5 200 -3.5 0 20");
          oneCell(new BlockPos(15, 200, 3), Blocks.COBBLESTONE.defaultBlockState());
        },
        RustySchematics::ready,
        20);
    add(
        "Manual keyboard input immediately relinquishes controls",
        () -> {
          expect(SchematicConstruction.start(), "Manual run starts");
          mc().options.keyUp.setDown(true);
        },
        () -> {
          boolean stopped = !SchematicConstruction.active();
          mc().options.keyUp.setDown(false);
          return stopped && SchematicConstruction.result.contains("Manual");
        },
        2);
    add(
        "Menus stop and never automatically resume",
        () -> {
          expect(SchematicConstruction.start(), "Menu run starts");
          mc().gui.setScreen(new RustyScreen());
        },
        () -> {
          if (SchematicConstruction.active()) return false;
          mc().gui.setScreen(null);
          return true;
        },
        3);
    addDamageCase(false);
    addDamageCase(true);
    add(
        "Health below12 stops immediately",
        () ->
            mc().getSingleplayerServer()
                .execute(() ->
                    mc().getSingleplayerServer()
                        .getPlayerList()
                        .getPlayers()
                        .forEach(p -> {
                          p.removeAllEffects();
                          // Keep food above its safety floor but below natural-regeneration range.
                          // Saturation20 otherwise heals11 to12 before this step's10-tick check.
                          p.getFoodData().setFoodLevel(17);
                          p.getFoodData().setSaturation(0);
                          p.setHealth(11);
                        })),
        () -> mc().player.getHealth() < 12,
        10);
    add(
        "Low-health start fails safety",
        () -> expect(!SchematicConstruction.start(), "Unsafe health refused"),
        () -> !SchematicConstruction.active(),
        2);
    add(
        "Food below10 stops immediately",
        () ->
            mc().getSingleplayerServer()
                .execute(() ->
                    mc().getSingleplayerServer()
                        .getPlayerList()
                        .getPlayers()
                        .forEach(p -> {
                          p.setHealth(20);
                          p.getFoodData().setFoodLevel(9);
                        })),
        () -> mc().player.getHealth() == 20 && mc().player.getFoodData().getFoodLevel() < 10,
        15);
    add(
        "Low-food start fails safety",
        () -> expect(!SchematicConstruction.start(), "Unsafe food refused"),
        () -> !SchematicConstruction.active(),
        2);
    add(
        "World transition setup heals before final scope check",
        () -> {
          command("effect give @a saturation 1 10");
          command("effect give @a instant_health 1 10");
        },
        () -> mc().player.getFoodData().getFoodLevel() >= 10 && mc().player.getHealth() >= 12,
        15);
    addFinalSurveyCases();
    add(
        "Dimension transition stops, clears transient selection and never resumes",
        () -> {
          expect(SchematicConstruction.start(), "World-change run starts");
          command("execute in minecraft:the_nether run tp @a 0 200 0");
        },
        () ->
            !SchematicConstruction.active()
                && !RustySchematics.loaded()
                && RustySchematics.first() == null
                && RustySchematics.second() == null,
        40);
  }

  private static net.minecraft.network.protocol.game.ClientboundLevelChunkPacketData originalColumn,
      changedColumn;
  private static final BlockPos SURVEY_CELL = new BlockPos(0, 220, 0);
  private static int churnChanges;
  private static final Map<
          net.minecraft.world.level.ChunkPos,
          net.minecraft.network.protocol.game.ClientboundLevelChunkPacketData>
      cacheSnapshots = new HashMap<>();

  private static void restoreSurveyCache() {
    var cache = mc().level.getChunkSource();
    cache.updateViewRadius(12);
    cache.updateViewCenter(
        mc().player.blockPosition().getX() >> 4, mc().player.blockPosition().getZ() >> 4);
    for (var e : cacheSnapshots.entrySet())
      cache.replaceWithPacketData(e.getKey().x(), e.getKey().z(), e.getValue());
  }

  private static void recordOutcome(String name) {
    outcomeEvidence.add(
        Map.of(
            "case",
            name,
            "outcome",
            SchematicConstruction.outcome,
            "placed",
            SchematicConstruction.placed,
            "sourced",
            SchematicConstruction.sourced,
            "walked",
            SchematicConstruction.walked,
            "active",
            SchematicConstruction.active()));
  }

  private static void addFinalSurveyCases() {
    add(
        "Mixed terminal outcome owned world setup",
        () -> {
          clearControllers();
          command("fill -2 200 -2 16 204 15 air");
          command("setblock 3 200 1 stone");
          command("setblock 3 200 3 bedrock");
          command("clear @a");
          command("item replace entity @a hotbar.0 with red_bed");
          command("tp @a 0.5 200 0.5 0 20");
        },
        () ->
            mc().player.onGround()
                && mc().player.getInventory().getItem(0).getItem().toString().contains("bed"),
        20);
    add(
        "Mixed preexisting, coupled, occupied, material, manual and data-detail plan",
        () -> {
          var cells =
              List.of(
                  new Schematic.Cell(
                      new Schematic.Pos(3, 0, 1), Schematic.State.parse("minecraft:stone")),
                  new Schematic.Cell(
                      new Schematic.Pos(0, 0, 3),
                      Schematic.State.parse(
                          "minecraft:red_bed[facing=south,part=foot,occupied=false]")),
                  new Schematic.Cell(
                      new Schematic.Pos(0, 0, 4),
                      Schematic.State.parse(
                          "minecraft:red_bed[facing=south,part=head,occupied=false]")),
                  new Schematic.Cell(
                      new Schematic.Pos(3, 0, 3), Schematic.State.parse("minecraft:stone")),
                  new Schematic.Cell(
                      new Schematic.Pos(10, 0, 3),
                      Schematic.State.parse("minecraft:diamond_block")),
                  new Schematic.Cell(
                      new Schematic.Pos(12, 0, 3),
                      Schematic.State.parse("minecraft:water[level=0]")));
          RustySchematics.accept(
              new Schematic.Document(cells, 3, "Owned mixed states-only outcome"));
          origin(0, 200, 0);
        },
        RustySchematics::ready,
        5);
    add(
        "Mixed result distinguishes one ordinary action from two received completed cells",
        () -> expect(SchematicConstruction.start(), "Mixed run starts"),
        () -> {
          var o = SchematicConstruction.outcome;
          if (SchematicConstruction.active() || o == null || !o.surveyComplete()) return false;
          expect(
              o.initiallyMatching() == 1 && o.newlyCompleted() == 2 && o.placementActions() == 1,
              "Preexisting versus received coupled cells versus actions");
          expect(
              o.finallyMatching() == 3
                  && o.occupied() == 1
                  && o.remainingMissing() == 2
                  && o.manual() == 1
                  && o.unknown() == 0
                  && o.importedDetails() == 3,
              "All final categories and imported detail count retained");
          recordOutcome("Mixed received outcome");
          return true;
        },
        5);
    add(
        "Late already-dispatched receipt setup",
        () -> {
          command("item replace entity @a hotbar.0 with cobblestone 4");
          command("tp @a 7.5 200 -1.5 0 20");
          oneCell(new BlockPos(8, 200, 1), Blocks.COBBLESTONE.defaultBlockState());
        },
        () ->
            RustySchematics.ready() && mc().player.getInventory().getItem(0).is(Items.COBBLESTONE),
        20);
    add(
        "Stop while actual ordinary use waits for its received acknowledgment",
        () -> expect(SchematicConstruction.start(), "Late receipt run starts"),
        () -> {
          if (!Boolean.TRUE.equals(SchematicConstruction.status().get("placementPending")))
            return false;
          SchematicConstruction.stop("Owned stop before late receipt");
          expect(
              SchematicConstruction.outcome.receiptPending()
                  && !SchematicConstruction.outcome.surveyComplete(),
              "Pending receipt is explicit at stop");
          return true;
        },
        1);
    add(
        "Late received placement refreshes outcome without another action",
        () -> {},
        () -> {
          var o = SchematicConstruction.outcome;
          if (o == null || o.receiptPending() || !o.surveyComplete()) return false;
          expect(
              !SchematicConstruction.active()
                  && o.newlyCompleted() == 1
                  && o.placementActions() == 1,
              "Only already dispatched action settled after stop");
          recordOutcome("Late actual received action after Stop");
          return true;
        },
        10);
    add(
        "Large received multi-batch survey setup",
        () -> {
          clearControllers();
          command("fill 0 220 0 15 235 15 stone");
          command("tp @a 7.5 200 -3.5 0 20");
          var cells = new ArrayList<Schematic.Cell>();
          for (int y = 0; y < 16; y++)
            for (int z = 0; z < 16; z++)
              for (int x = 0; x < 16; x++)
                cells.add(
                    new Schematic.Cell(
                        new Schematic.Pos(x, y, z), Schematic.State.parse("minecraft:stone")));
          RustySchematics.accept(new Schematic.Document(cells, 0, "Owned multi-batch survey"));
          origin(0, 220, 0);
        },
        () ->
            RustySchematics.ready()
                && mc().level.getBlockState(new BlockPos(15, 235, 15)).is(Blocks.STONE),
        20);
    add(
        "Prepare explicit controlled host-cache replacement snapshots",
        () -> {
          cacheSnapshots.clear();
          for (int x = -1; x <= 1; x++)
            for (int z = -1; z <= 1; z++) {
              var chunk =
                  mc().level
                      .getChunkSource()
                      .getChunk(
                          x, z, net.minecraft.world.level.chunk.status.ChunkStatus.FULL, false);
              if (chunk != null)
                cacheSnapshots.put(
                    new net.minecraft.world.level.ChunkPos(x, z),
                    new net.minecraft.network.protocol.game.ClientboundLevelChunkPacketData(chunk));
            }
          originalColumn = cacheSnapshots.get(new net.minecraft.world.level.ChunkPos(0, 0));
          mc().level
              .setServerVerifiedBlockState(SURVEY_CELL, Blocks.BEDROCK.defaultBlockState(), 19);
          changedColumn =
              new net.minecraft.network.protocol.game.ClientboundLevelChunkPacketData(
                  mc().level.getChunk(0, 0));
          restoreSurveyCache();
        },
        () -> mc().level.getBlockState(SURVEY_CELL).is(Blocks.STONE),
        1);
    for (String kind : List.of("block", "column", "unload", "center", "radius")) {
      add(
          "Begin controlled host-cache " + kind + " invalidation after first batch",
          () -> {
            restoreSurveyCache();
            if (kind.equals("radius")) mc().level.getChunkSource().updateViewCenter(6, 0);
            expect(SchematicConstruction.start(), "Multi-batch run starts");
          },
          () -> {
            if (!SchematicConstruction.active()
                || ((Number) SchematicConstruction.status().get("surveyScanned")).intValue() < 2048)
              return false;
            var cache = mc().level.getChunkSource();
            switch (kind) {
              case "block" ->
                  mc().level
                      .setServerVerifiedBlockState(
                          SURVEY_CELL, Blocks.BEDROCK.defaultBlockState(), 19);
              case "column" -> cache.replaceWithPacketData(0, 0, changedColumn);
              case "unload" -> cache.drop(new net.minecraft.world.level.ChunkPos(0, 0));
              case "center" -> cache.updateViewCenter(100, 100);
              case "radius" -> cache.updateViewRadius(2);
            }
            return true;
          },
          1);
      add(
          "No stale Complete after controlled host-cache " + kind,
          () -> {},
          () -> {
            if (SchematicConstruction.active()) return false;
            expect(!stopped("Complete:"), "Already-scanned mutation must invalidate completion");
            expect(
                SchematicConstruction.placed == 0
                    && SchematicConstruction.sourced == 0
                    && SchematicConstruction.walked == 0,
                "Survey case never acts");
            restoreSurveyCache();
            return true;
          },
          2);
      add(
          "Stable read-only refresh after controlled host-cache " + kind,
          () -> {},
          () -> {
            var o = SchematicConstruction.outcome;
            if (!SchematicConstruction.active() && o != null && o.surveyComplete()) {
              recordOutcome("Controlled host-cache " + kind + " readback");
              return true;
            }
            return false;
          },
          3);
    }
    add(
        "Stable second large survey can complete",
        () -> {
          restoreSurveyCache();
          expect(SchematicConstruction.start(), "Stable run starts");
        },
        () ->
            stopped("Complete:")
                && SchematicConstruction.outcome != null
                && SchematicConstruction.outcome.surveyComplete(),
        5);
    add(
        "Continuous controlled target changes have a finite survey bound",
        () -> {
          restoreSurveyCache();
          churnChanges = 0;
          expect(SchematicConstruction.start(), "Churn run starts");
        },
        () -> {
          if (!SchematicConstruction.active()) {
            expect(
                stopped("did not stabilize") && churnChanges <= 12,
                "Churn stops explicitly unverified within restart bound");
            restoreSurveyCache();
            return true;
          }
          mc().level
              .setServerVerifiedBlockState(
                  SURVEY_CELL,
                  (++churnChanges % 2 == 0 ? Blocks.STONE : Blocks.BEDROCK).defaultBlockState(),
                  19);
          return false;
        },
        1);
    add(
        "Cancellation during multi-batch scan labels initial and final uncertainty",
        () -> {
          restoreSurveyCache();
          expect(SchematicConstruction.start(), "Partial stop run starts");
        },
        () -> {
          if (((Number) SchematicConstruction.status().get("surveyScanned")).intValue() < 2048)
            return false;
          SchematicConstruction.stop("Owned stop mid-survey");
          var o = SchematicConstruction.outcome;
          expect(
              o.initiallyMatching() == null && !o.surveyComplete() && o.unknown() >= 2048,
              "Unscanned cells remain explicitly unknown");
          mc().level
              .setServerVerifiedBlockState(SURVEY_CELL, Blocks.BEDROCK.defaultBlockState(), 19);
          return true;
        },
        1);
    add(
        "Stopped survey readback is coherent and never rearms",
        () -> {},
        () -> {
          var o = SchematicConstruction.outcome;
          if (o == null || !o.surveyComplete()) return false;
          expect(
              !SchematicConstruction.active()
                  && o.finallyMatching() == 4095
                  && o.occupied() == 1
                  && o.newlyCompleted() == 0
                  && o.placementActions() == 0,
              "Readback categories exclude all automatic work");
          recordOutcome("Stopped multi-batch readback");
          restoreSurveyCache();
          oneCell(new BlockPos(15, 200, 3), Blocks.COBBLESTONE.defaultBlockState());
          return true;
        },
        5);
  }

  private static void addDamageCase(boolean absorbed) {
    String kind = absorbed ? "Absorbed damage" : "Damage";
    add(
        kind + " owned healthy received baseline",
        () -> {
          clearControllers();
          command("effect clear @a");
          command("effect give @a instant_health 1 10");
          command("effect give @a saturation 1 10");
          if (absorbed) command("effect give @a absorption 60 0");
          command("tp @a 7.5 200 -3.5 0 20");
        },
        () -> {
          expect(
              mc().level.getBlockState(new BlockPos(15, 200, 3)).isAir(),
              "Damage target remains unplaced");
          return mc().player.getHealth() == 20
              && (absorbed
                  ? mc().player.getAbsorptionAmount() >= 4
                  : mc().player.getAbsorptionAmount() == 0)
              && mc().player.hurtTime == 0
              && mc().player.damageCooldownTime == 0
              && mc().player.onGround();
        },
        40);
    add(
        kind + " active construction baseline before server command",
        () -> {
          expect(SchematicConstruction.start(), kind + " run starts");
          SchematicDamageEvidence.begin(absorbed);
        },
        SchematicDamageEvidence::activeBaselineEstablished,
        2);
    add(
        kind + " stops before a new placement",
        SchematicDamageEvidence::apply,
        SchematicDamageEvidence::stoppedWithProof,
        5);
    add(
        kind + " never resumes or dispatches another item use",
        () -> {},
        SchematicDamageEvidence::finishNoResume,
        20);
  }

  private static void prepareDenied() {
    RustyClient.config = new Settings();
    RustyClient.set("build_planner", true);
    add(
        "Unmodified loopback server requires current-session view + movement + assistance",
        () -> {},
        () -> !SchematicConstruction.permitted(),
        40);
    add(
        "Explicit construction fails closed without grants",
        () -> expect(!SchematicConstruction.start(), "Denied server refuses start"),
        () -> !SchematicConstruction.active() && SchematicConstruction.placed == 0,
        5);
  }

  private static void write(boolean done, String error) {
    try {
      var receipt = new LinkedHashMap<String, Object>();
      receipt.put("finished", done);
      receipt.put("passed", done && error == null);
      receipt.put("error", error);
      receipt.put("checks", checks);
      receipt.put("construction", SchematicConstruction.status());
      receipt.put("outcomeEvidence", outcomeEvidence);
      receipt.put(
          "cacheFixtureKind", "controlled host-cache mutation; not observed network delivery");
      receipt.put("multiMaterialPlacements", completedPlacements);
      receipt.put("ordinaryLookUses", SchematicLookEvidence.uses);
      receipt.put("ordinaryLookViolations", SchematicLookEvidence.violations);
      receipt.put("ordinaryLookTrace", SchematicLookEvidence.trace());
      receipt.put("damageEvidence", SchematicDamageEvidence.receipt());
      receipt.put("uiEvidence", SchematicUiEvidence.receipt());
      receipt.put("export", export == null ? null : export.getFileName().toString());
      Files.writeString(
          mc().gameDirectory.toPath().resolve("rusty-qa-schematics.json"),
          RustyClient.GSON.toJson(receipt));
    } catch (Exception e) {
      RustyClient.LOG.error("Could not write schematic receipt", e);
    }
  }

  private static void finish(String error) {
    finished = true;
    write(true, error);
    clearControllers();
    if (oldClipboard != null) mc().keyboardHandler.setClipboard(oldClipboard);
    if (!Boolean.getBoolean("rusty.qa.keepOpen")) mc().stop();
  }
}
