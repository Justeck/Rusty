package dev.justeck.rusty;

import java.util.*;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.*;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;

public final class TargetScreen extends Screen {
  private EditBox input;
  private int page;
  private String message = "Use a full block ID, for example minecraft:diamond_ore";
  private final Screen parent;

  public TargetScreen() {
    this(new RustyScreen());
  }

  public TargetScreen(Screen parent) {
    super(Component.literal("Selected blocks"));
    this.parent = parent;
  }

  private void button(String label, int x, int y, int w, Runnable action) {
    addRenderableWidget(
        Button.builder(Component.literal(label), b -> action.run()).bounds(x, y, w, 20).build());
  }

  @Override
  protected void init() {
    int x = width / 2 - 170;
    input = addRenderableWidget(new EditBox(font, x, 48, 260, 20, Component.literal("Block ID")));
    input.setMaxLength(256);
    button(
        "Add",
        x + 266,
        48,
        74,
        () -> {
          try {
            String id = input.getValue().trim();
            var key = Identifier.parse(id);
            if (!BuiltInRegistries.BLOCK.containsKey(key))
              throw new IllegalArgumentException("Unknown block ID");
            if (RustyClient.config.targets.size() >= 256)
              throw new IllegalArgumentException("Maximum 256 selected blocks");
            RustyClient.config.targets.add(key.toString());
            RustyClient.save();
            message = "Added " + key;
            rebuildWidgets();
          } catch (Exception e) {
            message = e.getMessage();
          }
        });
    int rows = Math.max(2, (height - 150) / 24);
    var list = new ArrayList<>(RustyClient.config.targets);
    page = Math.min(page, Math.max(0, (list.size() - 1) / rows));
    for (int i = 0; i < rows; i++) {
      int index = page * rows + i;
      if (index >= list.size()) break;
      String id = list.get(index);
      button(
          "Remove  " + id,
          x,
          82 + i * 24,
          340,
          () -> {
            RustyClient.config.targets.remove(id);
            RustyClient.save();
            rebuildWidgets();
          });
    }
    button(
        "<",
        x,
        height - 58,
        34,
        () -> {
          page = Math.max(0, page - 1);
          rebuildWidgets();
        });
    button(
        ">",
        x + 40,
        height - 58,
        34,
        () -> {
          if ((page + 1) * rows < list.size()) page++;
          rebuildWidgets();
        });
    button(
        "Typical ores",
        x + 80,
        height - 58,
        116,
        () -> {
          RustyClient.config.targets = dev.justeck.rusty.core.Settings.defaultTargets();
          RustyClient.save();
          rebuildWidgets();
        });
    button("Done", x + 220, height - 58, 120, this::onClose);
  }

  @Override
  public void extractRenderState(GuiGraphicsExtractor g, int x, int y, float delta) {
    g.fill(0, 0, width, height, 0xef24211d);
    g.text(font, "SELECTED BLOCKS", width / 2 - 170, 20, 0xffdda167);
    g.text(font, font.plainSubstrByWidth(message, 340), width / 2 - 170, height - 25, 0xffbbb2a5);
    super.extractRenderState(g, x, y, delta);
  }

  @Override
  public void onClose() {
    minecraft.gui.setScreen(parent);
  }

  @Override
  public boolean isPauseScreen() {
    return false;
  }
}
