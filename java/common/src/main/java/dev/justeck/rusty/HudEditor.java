package dev.justeck.rusty;

import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;

public final class HudEditor extends Screen {
  private String dragging, selected;
  private double offsetX, offsetY;
  private final Screen parent;

  public HudEditor() {
    this(new RustyScreen());
  }

  public HudEditor(Screen parent) {
    super(Component.literal("Rusty HUD layout"));
    this.parent = parent;
  }

  @Override
  protected void init() {
    addRenderableWidget(
        Button.builder(
                Component.literal("Default panel"),
                b -> {
                  if (selected != null) {
                    RustyClient.config.overlays.remove(selected);
                    RustyClient.config.overlayScales.remove(selected);
                    RustyClient.save();
                  }
                })
            .bounds(width / 2 - 166, height - 30, 104, 20)
            .build());
    addRenderableWidget(
        Button.builder(
                Component.literal("Default layout"),
                b -> {
                  RustyClient.config.overlays.clear();
                  RustyClient.config.overlayScales.clear();
                  RustyClient.save();
                })
            .bounds(width / 2 - 56, height - 30, 104, 20)
            .build());
    addRenderableWidget(
        Button.builder(Component.literal("Done"), b -> onClose())
            .bounds(width / 2 + 54, height - 30, 104, 20)
            .build());
  }

  @Override
  public void extractRenderState(GuiGraphicsExtractor g, int x, int y, float delta) {
    g.fill(0, 0, width, height, 0x66141414);
    RustyHud.draw(g, true);
    g.text(
        font,
        "Select and drag to move. Scroll to resize. Default panel resets the selection.",
        Math.max(4, width / 2 - 220),
        height - 44,
        0xffe0b27b);
    super.extractRenderState(g, x, y, delta);
  }

  @Override
  public boolean mouseClicked(MouseButtonEvent event, boolean twice) {
    if (super.mouseClicked(event, twice)) return true;
    for (var entry : RustyHud.bounds.entrySet()) {
      var r = entry.getValue();
      if (event.x() >= r[0]
          && event.x() < r[0] + r[2]
          && event.y() >= r[1]
          && event.y() < r[1] + r[3]) {
        dragging = selected = entry.getKey();
        offsetX = event.x() - r[0];
        offsetY = event.y() - r[1];
        return true;
      }
    }
    return false;
  }

  @Override
  public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) {
    if (dragging != null) {
      var p = RustyClient.config.overlays.computeIfAbsent(dragging, k -> new double[] {.02, .02});
      p[0] = Math.clamp((event.x() - offsetX) / width, 0, .95);
      p[1] = Math.clamp((event.y() - offsetY) / height, 0, .92);
      return true;
    }
    return super.mouseDragged(event, dx, dy);
  }

  @Override
  public boolean mouseReleased(MouseButtonEvent event) {
    dragging = null;
    RustyClient.save();
    return super.mouseReleased(event);
  }

  @Override
  public boolean mouseScrolled(double x, double y, double horizontal, double vertical) {
    for (var entry : RustyHud.bounds.entrySet()) {
      var r = entry.getValue();
      if (x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]) {
        var scales = RustyClient.config.overlayScales;
        scales.put(
            entry.getKey(),
            Math.clamp(scales.getOrDefault(entry.getKey(), 1.0) + vertical * .1, .5, 2));
        RustyClient.save();
        return true;
      }
    }
    return false;
  }

  @Override
  public boolean isPauseScreen() {
    return false;
  }

  @Override
  public void onClose() {
    RustyClient.save();
    minecraft.gui.setScreen(parent);
  }
}
