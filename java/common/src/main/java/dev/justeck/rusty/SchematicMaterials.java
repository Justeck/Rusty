package dev.justeck.rusty;

import dev.justeck.rusty.core.Schematic;
import java.util.*;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.*;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.BlockState;

/** Runtime registry mapping; state totals remain visible even for manual-only requirements. */
public final class SchematicMaterials {
  public record Requirement(Item item, int units, String manual) {}

  public static Requirement requirement(BlockState state) {
    var descriptor = RustySchematics.describe(state);
    int units = Schematic.materialUnits(descriptor);
    if (descriptor.skipped()) return new Requirement(Items.AIR, 0, "");
    String id = descriptor.id();
    if (!id.startsWith("minecraft:"))
      return new Requirement(
          state.getBlock().asItem(),
          units,
          "Custom block write behavior requires manual placement");
    Item item = state.getBlock().asItem();
    if (state.getBlock() instanceof FlowerPotBlock pot && state.getBlock() != Blocks.FLOWER_POT)
      return new Requirement(
          Items.FLOWER_POT, 1, "Potted plant: flower pot + plant, manual placement");
    if (id.contains("cauldron") && state.getBlock() != Blocks.CAULDRON)
      return new Requirement(Items.CAULDRON, 1, "Filled cauldron: contents require manual use");
    if (item == Items.AIR || !(item instanceof BlockItem))
      return new Requirement(item, Math.max(1, units), "No ordinary block item; manual state");
    if (!automaticBlock(state))
      return new Requirement(
          item, units, "Active/special block callbacks require manual placement");
    if (!automaticItem(item))
      return new Requirement(item, units, "Special item placement requires manual work");
    return new Requirement(item, units, "");
  }

  public static boolean automaticBlock(BlockState state) {
    var block = state.getBlock();
    return block.getClass() == Block.class
        || block instanceof RotatedPillarBlock
        || block instanceof TransparentBlock
        || block instanceof StainedGlassBlock
        || block instanceof TintedGlassBlock
        || block instanceof SlabBlock
        || block instanceof StairBlock
        || block instanceof DoorBlock
        || block instanceof TrapDoorBlock
        || block instanceof ChestBlock
        || block instanceof BarrelBlock
        || block instanceof AbstractBedBlock
        || block instanceof DoublePlantBlock
        || block instanceof SnowLayerBlock
        || block instanceof CandleBlock
        || block instanceof TurtleEggBlock
        || block instanceof SeaPickleBlock
        || block instanceof FenceBlock
        || block instanceof WallBlock
        || block instanceof IronBarsBlock
        || block instanceof CarpetBlock
        || block instanceof LadderBlock
        || block instanceof DropExperienceBlock;
  }

  public static boolean automaticItem(Item item) {
    return Set.of(
            "net.minecraft.world.item.BlockItem",
            "net.minecraft.world.item.DoubleHighBlockItem",
            "net.minecraft.world.item.BedItem",
            "net.minecraft.world.item.StandingAndWallBlockItem")
        .contains(item.getClass().getName());
  }

  public static String text(Schematic.Document doc, Map<Schematic.State, BlockState> resolved) {
    var items = new TreeMap<String, Long>();
    var states = new TreeMap<String, Long>();
    var manual = new TreeMap<String, String>();
    for (var cell : doc.cells()) {
      var descriptor = cell.state();
      states.merge(descriptor.canonical(), 1L, Long::sum);
      var state = resolved.get(descriptor);
      var r = requirement(state);
      if (r.units > 0 && r.item != Items.AIR)
        items.merge(BuiltInRegistries.ITEM.getKey(r.item).toString(), (long) r.units, Long::sum);
      if (state.getBlock() instanceof FlowerPotBlock pot && state.getBlock() != Blocks.FLOWER_POT) {
        var plant = pot.getPotted().asItem();
        if (plant != Items.AIR)
          items.merge(BuiltInRegistries.ITEM.getKey(plant).toString(), 1L, Long::sum);
      }
      if (!r.manual.isEmpty()) manual.put(descriptor.canonical(), r.manual);
    }
    var out = new StringBuilder("Rusty schematic materials — full plan totals\n");
    out.append("States only; block-entity/entity/tick details not restored: ")
        .append(doc.detailCount())
        .append("\n\nItems (quantity; stacks + remainder; have; missing)\n");
    var mc = net.minecraft.client.Minecraft.getInstance();
    for (var e : items.entrySet()) {
      var item = BuiltInRegistries.ITEM.getValue(Identifier.parse(e.getKey()));
      int max = item.getDefaultMaxStackSize(), have = 0;
      if (mc.player != null)
        for (int i = 0; i < 36; i++) {
          var stack = mc.player.getInventory().getItem(i);
          if (stack.is(item)) have += stack.getCount();
        }
      out.append(item.getName(item.getDefaultInstance()).getString())
          .append(" [")
          .append(e.getKey())
          .append("]: ")
          .append(e.getValue())
          .append("; ")
          .append(e.getValue() / max)
          .append(" stacks + ")
          .append(e.getValue() % max)
          .append("; have ")
          .append(have)
          .append("; missing ")
          .append(Math.max(0, e.getValue() - have))
          .append('\n');
    }
    out.append("\nExact block-state totals (including air/skipped cells)\n");
    states.forEach((state, n) -> out.append(state).append(": ").append(n).append('\n'));
    if (!manual.isEmpty()) {
      out.append("\nManual requirements\n");
      manual.forEach((state, reason) -> out.append(state).append(": ").append(reason).append('\n'));
    }
    return out.toString();
  }
}
