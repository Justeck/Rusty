package dev.justeck.rusty;

import java.util.*;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

/** Paginated readable exact state/item totals; Copy always uses the operating-system clipboard. */
public final class SchematicMaterialScreen extends Screen {
  private final Screen parent;
  private final List<String> lines;
  private int page;
  private final boolean outcome;
  private List<net.minecraft.util.FormattedCharSequence> wrapped = List.of();

  public SchematicMaterialScreen(Screen parent) {
    this(parent, false);
  }

  public SchematicMaterialScreen(Screen parent, boolean outcome) {
    super(Component.literal(outcome ? "Construction result" : "Schematic materials"));
    this.parent = parent;
    this.outcome = outcome;
    this.lines =
        (outcome ? SchematicConstruction.details() : RustySchematics.materialText())
            .lines()
            .toList();
  }

  private void button(String label, int x, int y, int w, Runnable r) {
    addRenderableWidget(
        Button.builder(Component.literal(label), b -> r.run()).bounds(x, y, w, 20).build());
  }

  private int rows() {
    return Math.max(1, (height - 100) / 12);
  }

  protected void init() {
    wrapped =
        lines.stream()
            .flatMap(line -> font.split(Component.literal(line), Math.max(40, width - 40)).stream())
            .toList();
    page = Math.min(page, Math.max(0, (wrapped.size() - 1) / rows()));
    int x = width / 2 - 170;
    button(
        "<",
        x,
        height - 38,
        35,
        () -> {
          page = Math.max(0, page - 1);
        });
    button(
        ">",
        x + 40,
        height - 38,
        35,
        () -> {
          if ((page + 1) * rows() < wrapped.size()) page++;
        });
    button(
        outcome ? "Copy outcome" : "Copy all materials",
        x + 80,
        height - 38,
        145,
        () -> {
          minecraft.keyboardHandler.setClipboard(String.join("\n", lines));
          RustyClient.notice =
              outcome
                  ? "Copied construction outcome"
                  : "Copied displayed material and state totals";
        });
    button("Done", x + 230, height - 38, 110, this::onClose);
  }

  public void extractRenderState(GuiGraphicsExtractor g, int x, int y, float delta) {
    g.fill(0, 0, width, height, 0xef24211d);
    g.text(
        font,
        (outcome ? "CONSTRUCTION RESULT — page " : "SCHEMATIC MATERIALS — page ") + (page + 1),
        20,
        18,
        0xffdda167);
    int start = page * rows();
    for (int i = 0; i < rows() && start + i < wrapped.size(); i++)
      g.text(font, wrapped.get(start + i), 20, 44 + i * 12, 0xffddd5ca);
    super.extractRenderState(g, x, y, delta);
  }

  public void onClose() {
    minecraft.gui.setScreen(parent);
  }

  public boolean isPauseScreen() {
    return false;
  }
}
