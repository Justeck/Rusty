package dev.justeck.rusty;

import com.mojang.blaze3d.platform.InputConstants;
import dev.justeck.rusty.core.HudGesture;
import java.util.*;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;

/** Draft-only visual layout editor; opening it never enables a feature. */
public final class HudEditor extends Screen {
  private String dragging, selected;
  private double startX, startY, startScale;
  private double[] startRect;
  private int corner = -1;
  private final Screen parent;
  final Map<String, double[]> positions = new LinkedHashMap<>();
  final Map<String, Double> scales = new LinkedHashMap<>();

  public HudEditor() {
    this(new RustyScreen());
  }

  public HudEditor(Screen parent) {
    super(Component.literal("Rusty HUD layout"));
    this.parent = parent;
    RustyClient.config.overlays.forEach((id, p) -> positions.put(id, p.clone()));
    scales.putAll(RustyClient.config.overlayScales);
  }

  @Override
  protected void init() {
    dragging = null;
    addRenderableWidget(
        Button.builder(
                Component.literal("Default panel"),
                b -> {
                  if (selected != null) {
                    positions.remove(selected);
                    scales.remove(selected);
                  }
                })
            .bounds(width / 2 - 214, height - 28, 104, 20)
            .build());
    addRenderableWidget(
        Button.builder(
                Component.literal("Default layout"),
                b -> {
                  positions.clear();
                  scales.clear();
                })
            .bounds(width / 2 - 106, height - 28, 104, 20)
            .build());
    addRenderableWidget(
        Button.builder(Component.literal("Done (Enter)"), b -> confirm())
            .bounds(width / 2 + 2, height - 28, 104, 20)
            .build());
    addRenderableWidget(
        Button.builder(Component.literal("Cancel (Esc)"), b -> onClose())
            .bounds(width / 2 + 110, height - 28, 104, 20)
            .build());
  }

  @Override
  public void extractBackground(GuiGraphicsExtractor g, int x, int y, float delta) {
    // Keep the live world clear while arranging overlays, without changing options.
    if (minecraft.level == null) super.extractBackground(g, x, y, delta);
  }

  @Override
  public void extractRenderState(GuiGraphicsExtractor g, int x, int y, float delta) {
    RustyHud.draw(g, true);
    var r = RustyHud.bounds.get(selected);
    if (r != null) {
      g.outline(r[0], r[1], r[2], r[3], 0xffffc77e);
      for (int c = 0; c < 4; c++) {
        int hx = r[0] + (c % 2 == 1 ? r[2] : 0), hy = r[1] + (c >= 2 ? r[3] : 0);
        g.fill(hx - 3, hy - 3, hx + 3, hy + 3, 0xff171411);
        g.fill(hx - 2, hy - 2, hx + 2, hy + 2, 0xffffc77e);
      }
    }
    String hint =
        RustyHud.bounds.isEmpty()
            ? "Enable a HUD feature, then edit its layout."
            : "Drag to move. Drag corners to resize. Enter saves; Esc cancels.";
    int tx = Math.max(4, (width - font.width(hint)) / 2);
    g.fill(tx - 4, height - 45, tx + font.width(hint) + 4, height - 31, 0xd01c1916);
    g.text(font, hint, tx, height - 42, 0xffeee7de);
    super.extractRenderState(g, x, y, delta);
  }

  private int handle(int[] r, double x, double y) {
    for (int c = 0; c < 4; c++)
      if (Math.abs(x - r[0] - (c % 2 == 1 ? r[2] : 0)) <= 4
          && Math.abs(y - r[1] - (c >= 2 ? r[3] : 0)) <= 4) return c;
    return -1;
  }

  @Override
  public boolean mouseClicked(MouseButtonEvent e, boolean twice) {
    if (super.mouseClicked(e, twice)) return true;
    if (e.button() != 0) return true;
    String picked = null;
    var chosen = RustyHud.bounds.get(selected);
    if (chosen != null && handle(chosen, e.x(), e.y()) >= 0) picked = selected;
    if (picked == null) {
      var entries = new ArrayList<>(RustyHud.bounds.entrySet());
      Collections.reverse(entries);
      for (var entry : entries) {
        var r = entry.getValue();
        if ((e.x() >= r[0] && e.x() <= r[0] + r[2] && e.y() >= r[1] && e.y() <= r[1] + r[3])
            || handle(r, e.x(), e.y()) >= 0) {
          picked = entry.getKey();
          break;
        }
      }
    }
    selected = dragging = picked;
    if (picked != null) {
      var r = RustyHud.bounds.get(picked);
      startRect = new double[] {r[0], r[1], r[2], r[3]};
      startX = e.x();
      startY = e.y();
      startScale = RustyHud.renderedScales.getOrDefault(picked, 1.0);
      corner = handle(r, e.x(), e.y());
    }
    return true;
  }

  @Override
  public boolean mouseDragged(MouseButtonEvent e, double dx, double dy) {
    if (dragging != null) {
      var result =
          corner < 0
              ? HudGesture.move(startRect, e.x() - startX, e.y() - startY, width, height)
              : HudGesture.resize(
                  startRect, startScale, corner, e.x() - startX, e.y() - startY, width, height);
      positions.put(dragging, new double[] {result.x() / width, result.y() / height});
      if (corner >= 0) scales.put(dragging, result.scale());
      return true;
    }
    return super.mouseDragged(e, dx, dy);
  }

  @Override
  public boolean mouseReleased(MouseButtonEvent e) {
    dragging = null;
    return super.mouseReleased(e);
  }

  @Override
  public boolean keyPressed(KeyEvent e) {
    if (e.key() == InputConstants.KEY_RETURN || e.key() == InputConstants.KEY_NUMPADENTER) {
      confirm();
      return true;
    }
    if (e.key() == InputConstants.KEY_ESCAPE) {
      onClose();
      return true;
    }
    return super.keyPressed(e);
  }

  private void confirm() {
    RustyClient.config.overlays.clear();
    positions.forEach((id, p) -> RustyClient.config.overlays.put(id, p.clone()));
    RustyClient.config.overlayScales.clear();
    RustyClient.config.overlayScales.putAll(scales);
    RustyClient.save();
    onClose();
  }

  @Override
  public boolean isPauseScreen() {
    return false;
  }

  @Override
  public void onClose() {
    minecraft.gui.setScreen(parent);
  }
}
