package dev.justeck.rusty;

import com.mojang.blaze3d.platform.InputConstants;
import dev.justeck.rusty.core.Catalog;
import dev.justeck.rusty.core.SessionPolicy;
import dev.justeck.rusty.core.Settings;
import java.util.*;
import java.util.function.*;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.*;
import net.minecraft.client.gui.screens.ConfirmScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.*;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;

/** Uses the game's widgets, text input and keyboard focus instead of an external GUI toolkit. */
public final class RustyScreen extends Screen {
  private String collection = "All", query = "", selected = "xray", capturing;
  private int page, detailPage;
  private double scale;
  private int w, h, left, right, top, panel, rows;
  private EditBox search;
  private final List<Button> rowButtons = new ArrayList<>(), rowToggles = new ArrayList<>();
  private final List<Catalog.Module> filtered = new ArrayList<>();
  private String popup;
  private final Screen parent;

  public RustyScreen() {
    this(null);
  }

  public RustyScreen(Screen parent) {
    super(Component.literal("Rusty"));
    this.parent = parent;
  }

  private void select(String id) {
    selected = id;
    detailPage = 0;
    RustyClient.config.recent.remove(id);
    RustyClient.config.recent.addFirst(id);
    RustyClient.save();
    rebuildWidgets();
  }

  private Button button(String text, int x, int y, int width, Consumer<Button> action) {
    return addRenderableWidget(
        Button.builder(Component.literal(text), action::accept).bounds(x, y, width, 20).build());
  }

  @Override
  protected void init() {
    scale = Settings.fittedMenuScale(width, height, RustyClient.config.menuScale);
    w = (int) (width / scale);
    h = (int) (height / scale);
    panel = Math.min(800, w - 24);
    left = (w - panel) / 2;
    right = left + panel / 2 + 8;
    top = 48;
    rows = Math.max(2, (h - 112) / 40);
    rows = Math.min(rows, 14);
    int half = panel / 2 - 16;
    search =
        addRenderableWidget(
            new EditBox(font, left + 8, top, half - 100, 20, Component.literal("Search modules")));
    search.setMaxLength(128);
    search.setValue(query);
    search.setResponder(s -> {
      query = s;
      page = 0;
      updateList();
    });
    search.setHint(Component.literal("Search tools..."));
    button(
        collection,
        left + half - 86,
        top,
        94,
        b -> {
          var collections = new ArrayList<>(List.of("All", "Enabled", "Favorites", "Recent"));
          Catalog.ALL.stream().map(Catalog.Module::category).distinct().forEach(collections::add);
          collection = collections.get((collections.indexOf(collection) + 1) % collections.size());
          page = 0;
          rebuildWidgets();
        });
    rowButtons.clear();
    rowToggles.clear();
    for (int i = 0; i < rows; i++) {
      final int row = i;
      int y = top + 30 + i * 40;
      rowButtons.add(
          button(
              "",
              left + 8,
              y,
              half - 48,
              b -> {
                var m = rowModule(row);
                if (m != null) select(m.id());
              }));
      rowToggles.add(
          button(
              "",
              left + half - 34,
              y,
              42,
              b -> {
                var m = rowModule(row);
                if (m != null) {
                  RustyClient.set(m.id(), !RustyClient.requested(m.id()));
                  updateList();
                }
              }));
    }
    button(
        "<",
        left + 8,
        h - 36,
        28,
        b -> {
          page = Math.max(0, page - 1);
          updateList();
        });
    button(
        ">",
        left + 42,
        h - 36,
        28,
        b -> {
          if ((page + 1) * rows < filtered.size()) page++;
          updateList();
        });
    button("HUD layout", left + 78, h - 36, 85, b -> minecraft.gui.setScreen(new HudEditor(this)));
    button(
        "Restore defaults",
        left + panel - 218,
        h - 36,
        124,
        b ->
            minecraft.gui.setScreen(
                new ConfirmScreen(
                    confirmed -> {
                      if (confirmed) {
                        RustyClient.restoreDefaults();
                        minecraft.gui.setScreen(new RustyScreen(parent));
                      } else minecraft.gui.setScreen(this);
                    },
                    Component.literal("Restore all Rusty settings?"),
                    Component.literal(
                        "Reset features, shortcuts, menu size and HUD layout. Saved waypoints and"
                            + " profile files are kept."))));
    button("Done", left + panel - 88, h - 36, 80, b -> onClose());
    updateList();
    details();
  }

  private Catalog.Module rowModule(int row) {
    int i = page * rows + row;
    return i < filtered.size() ? filtered.get(i) : null;
  }

  private void updateList() {
    filtered.clear();
    Catalog.ALL.stream()
        .filter(m -> m.matches(query))
        .filter(m ->
            collection.equals("All")
                || collection.equals("Enabled") && RustyClient.requested(m.id())
                || collection.equals("Favorites") && RustyClient.config.favorites.contains(m.id())
                || collection.equals("Recent") && RustyClient.config.recent.contains(m.id())
                || m.category().equals(collection))
        .forEach(filtered::add);
    page = Math.min(page, Math.max(0, (filtered.size() - 1) / Math.max(1, rows)));
    for (int i = 0; i < rowButtons.size(); i++) {
      var m = rowModule(i);
      var row = rowButtons.get(i);
      var toggle = rowToggles.get(i);
      row.visible = toggle.visible = m != null;
      if (m != null) {
        row.setMessage(Component.literal(m.name()));
        toggle.setMessage(
            Component.literal(
                m.id().equals("zoom") ? "KEY" : RustyClient.requested(m.id()) ? "ON" : "OFF"));
      }
    }
  }

  private void details() {
    var module =
        Catalog.ALL.stream().filter(m -> m.id().equals(selected)).findFirst().orElseThrow();
    int x = right, y = top + 28, bw = panel / 2 - 24;
    button(
        "?",
        right + bw - 24,
        top,
        24,
        b -> {
          popup =
              module.description()
                  + (SessionPolicy.required(selected).isEmpty()
                      ? ""
                      : " Multiplayer requires server permission: "
                          + SessionPolicy.required(selected)
                          + ".");
        });
    int clearWidth = 44, defaultWidth = 56, bindWidth = bw - clearWidth - defaultWidth - 12;
    int binding = RustyClient.config.bindings.getOrDefault(selected, -1);
    String bindingLabel =
        binding < 0
            ? "None"
            : InputConstants.Type.KEYBOARD.getOrCreate(binding).getDisplayName().getString();
    button(
        bindingLabel,
        x,
        y,
        bindWidth,
        b -> {
          capturing = selected;
          popup = "Press a key. Escape cancels; Backspace clears this shortcut.";
        });
    button("Default", x + bindWidth + 6, y, defaultWidth, b -> restoreBinding(selected));
    button(
        "Clear",
        x + bw - clearWidth,
        y,
        clearWidth,
        b -> {
          RustyClient.config.bindings.put(selected, -1);
          RustyClient.save();
          rebuildWidgets();
        });
    y += 30;
    final int controlY = y;
    var options = new ArrayList<Runnable>();
    var ys = new int[] {y};
    options.add(() -> {
      button(
          "Enabled: " + (RustyClient.requested(selected) ? "ON" : "OFF"),
          x,
          ys[0],
          bw - 62,
          b -> {
            RustyClient.set(selected, !RustyClient.requested(selected));
            rebuildWidgets();
          });
      button(
          "Default",
          x + bw - 56,
          ys[0],
          56,
          b -> {
            RustyClient.set(selected, new Settings().enabled.contains(selected));
            rebuildWidgets();
          });
      ys[0] += 26;
    });
    options.add(() -> {
      button(
          RustyClient.config.favorites.contains(selected) ? "Starred" : "Favorite",
          x,
          ys[0],
          bw - 62,
          b -> {
            if (!RustyClient.config.favorites.remove(selected))
              RustyClient.config.favorites.add(selected);
            RustyClient.save();
            rebuildWidgets();
          });
      button(
          "Default",
          x + bw - 56,
          ys[0],
          56,
          b -> {
            RustyClient.config.favorites.remove(selected);
            RustyClient.save();
            rebuildWidgets();
          });
      ys[0] += 26;
    });
    if (selected.equals("xray") || selected.equals("finder")) {
      if (selected.equals("xray")) {
        options.add(() ->
            slider(
                "textureVisibility",
                "Block textures",
                0,
                100,
                RustyClient.config.textureVisibility,
                v -> RustyClient.config.textureVisibility = v,
                x,
                ys));
        options.add(() ->
            toggle(
                "context",
                "Textured context",
                () -> RustyClient.config.context,
                v -> RustyClient.config.context = v,
                x,
                ys));
        options.add(() ->
            toggle(
                "holdXray",
                "Hold to activate",
                () -> RustyClient.config.holdXray,
                v -> RustyClient.config.holdXray = v,
                x,
                ys));
      }
      if (selected.equals("finder")) {
        options.add(() ->
            slider(
                "scanRadius",
                "Finder radius",
                8,
                128,
                RustyClient.config.scanRadius,
                v -> RustyClient.config.scanRadius = (int) v,
                x,
                ys));
        options.add(() ->
            toggle(
                "tracers",
                "Tracers",
                () -> RustyClient.config.tracers,
                v -> RustyClient.config.tracers = v,
                x,
                ys));
        options.add(() ->
            toggle(
                "outlines",
                "Boxes",
                () -> RustyClient.config.outlines,
                v -> RustyClient.config.outlines = v,
                x,
                ys));
      }
      options.add(() -> {
        button(
            "Selected blocks...",
            x,
            ys[0],
            bw,
            b -> minecraft.gui.setScreen(new TargetScreen(this)));
        ys[0] += 26;
      });
      options.add(() ->
          slider(
              "minY",
              "Minimum Y",
              -64,
              319,
              RustyClient.config.minY,
              v -> RustyClient.config.minY = (int) v,
              x,
              ys));
      options.add(() ->
          slider(
              "maxY",
              "Maximum Y",
              -64,
              319,
              RustyClient.config.maxY,
              v -> RustyClient.config.maxY = (int) v,
              x,
              ys));
      options.add(() ->
          slider(
              "maxTargets",
              "Maximum targets",
              16,
              4096,
              RustyClient.config.maxTargets,
              v -> RustyClient.config.maxTargets = (int) v,
              x,
              ys));
    } else if (selected.startsWith("entity_")) {
      options.add(() ->
          toggle(
              "espPlayers",
              "Players",
              () -> RustyClient.config.espPlayers,
              v -> RustyClient.config.espPlayers = v,
              x,
              ys));
      options.add(() ->
          toggle(
              "espMobs",
              "Mobs",
              () -> RustyClient.config.espMobs,
              v -> RustyClient.config.espMobs = v,
              x,
              ys));
      options.add(() ->
          toggle(
              "espItems",
              "Dropped items",
              () -> RustyClient.config.espItems,
              v -> RustyClient.config.espItems = v,
              x,
              ys));
    } else if (selected.equals("waypoints")
        || selected.equals("death_markers")
        || selected.equals("waypoint_walk")) {
      options.add(() -> {
        button("Save current position", x, ys[0], bw, b -> RustyNavigation.saveMarker());
        ys[0] += 26;
      });
      options.add(() -> {
        button("Select next waypoint", x, ys[0], bw, b -> RustyNavigation.nextMarker());
        ys[0] += 26;
      });
      options.add(() -> {
        button("Remove selected", x, ys[0], bw, b -> RustyNavigation.removeMarker());
        ys[0] += 26;
      });
      options.add(() -> {
        button(
            "Walk to selected",
            x,
            ys[0],
            bw,
            b -> {
              RustyNavigation.startWalk();
              onClose();
            });
        ys[0] += 26;
      });
    } else if (selected.equals("build_planner")) {
      options.add(() ->
          slider(
              "plannerWidth",
              "Width",
              1,
              16,
              RustyClient.config.plannerWidth,
              v -> RustyClient.config.plannerWidth = (int) v,
              x,
              ys));
      options.add(() ->
          slider(
              "plannerDepth",
              "Depth",
              1,
              16,
              RustyClient.config.plannerDepth,
              v -> RustyClient.config.plannerDepth = (int) v,
              x,
              ys));
      options.add(() ->
          slider(
              "plannerHeight",
              "Height",
              1,
              16,
              RustyClient.config.plannerHeight,
              v -> RustyClient.config.plannerHeight = (int) v,
              x,
              ys));
      options.add(() ->
          toggle(
              "plannerHollow",
              "Hollow",
              () -> RustyClient.config.plannerHollow,
              v -> RustyClient.config.plannerHollow = v,
              x,
              ys));
      options.add(() -> {
        button("Anchor at crosshair", x, ys[0], bw, b -> RustyPlanner.anchor());
        ys[0] += 26;
      });
      options.add(() -> {
        button(
            "Place one block",
            x,
            ys[0],
            bw,
            b -> {
              RustyPlanner.placeNext();
              onClose();
            });
        ys[0] += 26;
      });
    } else if (selected.equals("minimap")) {
      options.add(() ->
          slider(
              "mapSize",
              "Map size",
              48,
              192,
              RustyClient.config.mapSize,
              v -> RustyClient.config.mapSize = (int) v,
              x,
              ys));
      options.add(() ->
          toggle(
              "mapRound",
              "Round",
              () -> RustyClient.config.mapRound,
              v -> RustyClient.config.mapRound = v,
              x,
              ys));
      options.add(() ->
          toggle(
              "mapRotate",
              "Rotate",
              () -> RustyClient.config.mapRotate,
              v -> RustyClient.config.mapRotate = v,
              x,
              ys));
    } else if (selected.equals("freecam"))
      options.add(() ->
          slider(
              "cameraSpeed",
              "Camera speed",
              .05,
              4,
              RustyClient.config.cameraSpeed,
              v -> RustyClient.config.cameraSpeed = v,
              x,
              ys));
    else if (selected.equals("zoom"))
      options.add(() ->
          slider(
              "zoomFactor",
              "Magnification",
              1.5,
              12,
              RustyClient.config.zoomFactor,
              v -> RustyClient.config.zoomFactor = v,
              x,
              ys));
    else if (selected.equals("auto_eat"))
      options.add(() ->
          slider(
              "eatBelow",
              "Eat below",
              1,
              19,
              RustyClient.config.eatBelow,
              v -> RustyClient.config.eatBelow = (int) v,
              x,
              ys));
    else if (selected.equals("trigger_bot"))
      options.add(() ->
          toggle(
              "triggerPlayers",
              "Include players",
              () -> RustyClient.config.triggerPlayers,
              v -> RustyClient.config.triggerPlayers = v,
              x,
              ys));
    else if (selected.equals("auto_fish"))
      options.add(() ->
          slider(
              "fishDelay",
              "Reel delay (ticks)",
              2,
              20,
              RustyClient.config.fishDelay,
              v -> RustyClient.config.fishDelay = (int) v,
              x,
              ys));
    options.add(() ->
        slider(
            "menuScale",
            "Menu size (%)",
            50,
            300,
            Settings.menuPercent(RustyClient.config.menuScale),
            v -> RustyClient.config.menuScale = v / 100 * Settings.DEFAULT_MENU_SCALE,
            x,
            ys));
    options.add(() -> {
      button(
          "Menu size: reset to 100%",
          x,
          ys[0],
          bw,
          b -> {
            RustyClient.config.menuScale = Settings.DEFAULT_MENU_SCALE;
            RustyClient.changed();
            rebuildWidgets();
          });
      ys[0] += 26;
    });
    options.add(() ->
        slider(
            "hudScale",
            "HUD size",
            .5,
            2,
            RustyClient.config.hudScale,
            v -> RustyClient.config.hudScale = v,
            x,
            ys));
    options.add(() -> {
      button(
          "Theme: " + RustyClient.config.theme,
          x,
          ys[0],
          bw - 62,
          b -> {
            RustyClient.config.theme =
                switch (RustyClient.config.theme) {
                  case "Classic" -> "Copper";
                  case "Copper" -> "Graphite";
                  default -> "Classic";
                };
            RustyClient.save();
            rebuildWidgets();
          });
      defaultButton("theme", x, ys[0]);
      ys[0] += 26;
    });
    options.add(() -> {
      button(
          "Rebind menu",
          x,
          ys[0],
          bw - 62,
          b -> {
            capturing = "menu";
            popup = "Press a key for the menu. Escape cancels; Backspace clears.";
          });
      button("Default", x + bw - 56, ys[0], 56, b -> restoreBinding("menu"));
      ys[0] += 26;
    });
    options.add(() -> {
      button("Save quick profile", x, ys[0], bw, b -> RustyClient.exportProfile("quick"));
      ys[0] += 26;
    });
    options.add(() -> {
      button(
          "Load quick profile",
          x,
          ys[0],
          bw,
          b -> {
            RustyClient.importProfile("quick");
            rebuildWidgets();
          });
      ys[0] += 26;
    });
    int count = Math.max(2, (h - controlY - 72) / 26);
    detailPage = Math.min(detailPage, Math.max(0, (options.size() - 1) / count));
    options.stream().skip((long) detailPage * count).limit(count).forEach(Runnable::run);
    button(
        "<",
        x,
        h - 64,
        28,
        b -> {
          detailPage = Math.max(0, detailPage - 1);
          rebuildWidgets();
        });
    button(
        "More >",
        x + 34,
        h - 64,
        62,
        b -> {
          if ((detailPage + 1) * count < options.size()) detailPage++;
          rebuildWidgets();
        });
    button(
        "Apply size",
        x + 102,
        h - 64,
        bw - 102,
        b -> {
          RustyClient.save();
          rebuildWidgets();
        });
  }

  private void restoreBinding(String id) {
    RustyClient.config.bindings.put(id, RustyClient.defaultBinding(id));
    RustyClient.save();
    rebuildWidgets();
  }

  private void defaultButton(String property, int x, int y) {
    button(
        "Default",
        x + panel / 2 - 80,
        y,
        56,
        b -> {
          RustyClient.config.restoreField(property);
          RustyClient.save();
          rebuildWidgets();
        });
  }

  private void toggle(
      String property, String name, BooleanSupplier get, Consumer<Boolean> set, int x, int[] y) {
    button(
        name + ": " + (get.getAsBoolean() ? "ON" : "OFF"),
        x,
        y[0],
        panel / 2 - 86,
        b -> {
          set.accept(!get.getAsBoolean());
          RustyClient.save();
          rebuildWidgets();
        });
    defaultButton(property, x, y[0]);
    y[0] += 26;
  }

  private void slider(
      String property,
      String name,
      double min,
      double max,
      double initial,
      DoubleConsumer set,
      int x,
      int[] y) {
    addRenderableWidget(
        new AbstractSliderButton(
            x, y[0], panel / 2 - 86, 20, Component.empty(), (initial - min) / (max - min)) {
          {
            updateMessage();
          }

          @Override
          protected void updateMessage() {
            setMessage(
                Component.literal(
                    name
                        + ": "
                        + String.format(
                            Locale.ROOT, max <= 12 ? "%.2f" : "%.0f", min + value * (max - min))));
          }

          @Override
          protected void applyValue() {
            set.accept(min + value * (max - min));
            RustyClient.changed();
          }
        });
    defaultButton(property, x, y[0]);
    y[0] += 26;
  }

  @Override
  public void extractRenderState(GuiGraphicsExtractor g, int mx, int my, float delta) {
    g.fill(0, 0, width, height, 0x88100d0a);
    g.pose().pushMatrix();
    g.pose().scale((float) scale);
    int bg = RustyClient.config.theme.equals("Copper") ? 0xf02e241b : 0xf0242424;
    g.fill(left, 12, left + panel, h - 12, bg);
    g.outline(left, 12, panel, h - 24, 0xff8c8172);
    g.blit(
        Identifier.fromNamespaceAndPath("rusty", "icon.png"),
        left + 10,
        17,
        left + 38,
        45,
        0,
        1,
        0,
        1);
    g.text(font, "Keep your bearings.", left + 48, 25, 0xffbab4a9);
    var selectedModule =
        Catalog.ALL.stream().filter(m -> m.id().equals(selected)).findFirst().orElseThrow();
    g.text(font, selectedModule.name(), right, top + 5, 0xffeeeeee);
    for (int i = 0; i < rowButtons.size(); i++) {
      var m = rowModule(i);
      if (m != null)
        g.text(
            font,
            m.category() + (!RustyClient.POLICY.allows(m.id()) ? "  |  Server opt-in" : ""),
            left + 12,
            top + 54 + i * 40,
            0xffb5ada3);
    }
    super.extractRenderState(g, (int) (mx / scale), (int) (my / scale), delta);
    String hint = capturing != null ? "Waiting for key..." : RustyClient.notice;
    if (!hint.isEmpty())
      g.text(font, font.plainSubstrByWidth(hint, panel - 210), left + 174, h - 30, 0xffd6b990);
    if (popup != null) {
      int pw = Math.min(340, w - 40);
      var lines = font.split(Component.literal(popup), pw);
      int ph = lines.size() * 12 + (capturing == null ? 34 : 18),
          px = (w - pw) / 2,
          py = Math.max(20, (h - ph) / 2);
      g.fill(px - 8, py - 8, px + pw + 8, py + ph, 0xff171411);
      g.outline(px - 8, py - 8, pw + 16, ph + 8, 0xffc57b40);
      int y = py;
      for (var line : lines) {
        g.text(font, line, px, y, 0xffeeeeee);
        y += 12;
      }
      if (capturing == null) g.text(font, "Click anywhere to close", px, py + ph - 18, 0xffaaa296);
    }
    g.pose().popMatrix();
  }

  @Override
  public boolean keyPressed(KeyEvent event) {
    if (capturing != null) {
      if (event.key() != InputConstants.KEY_ESCAPE) {
        RustyClient.config.bindings.put(
            capturing, event.key() == InputConstants.KEY_BACKSPACE ? -1 : event.key());
        RustyClient.save();
      }
      capturing = null;
      popup = null;
      rebuildWidgets();
      return true;
    }
    if (popup != null) {
      popup = null;
      return true;
    }
    if (event.key() == RustyClient.config.bindings.getOrDefault("menu", -1)) {
      onClose();
      return true;
    }
    boolean handled = super.keyPressed(event);
    refreshMenuScale();
    return handled;
  }

  private MouseButtonEvent scaled(MouseButtonEvent e) {
    return new MouseButtonEvent(e.x() / scale, e.y() / scale, e.buttonInfo());
  }

  @Override
  public boolean mouseClicked(MouseButtonEvent e, boolean twice) {
    if (popup != null && capturing == null) {
      popup = null;
      return true;
    }
    return super.mouseClicked(scaled(e), twice);
  }

  @Override
  public boolean mouseReleased(MouseButtonEvent e) {
    boolean handled = super.mouseReleased(scaled(e));
    // Apply after the drag ends, keeping the slider under the pointer while dragging.
    refreshMenuScale();
    return handled;
  }

  private void refreshMenuScale() {
    if (Math.abs(scale - Settings.fittedMenuScale(width, height, RustyClient.config.menuScale))
        > 1e-6) rebuildWidgets();
  }

  @Override
  public boolean mouseDragged(MouseButtonEvent e, double dx, double dy) {
    return super.mouseDragged(scaled(e), dx / scale, dy / scale);
  }

  @Override
  public boolean isPauseScreen() {
    return false;
  }

  @Override
  public void onClose() {
    RustyClient.menuClosed();
    RustyClient.save();
    minecraft.gui.setScreen(parent);
  }
}
