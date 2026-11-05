package dev.justeck.rusty.core;

import java.util.*;

public final class HudLayout {
  public record Panel(String id, int width, int height) {}

  public record Rect(int x, int y, int width, int height) {
    public boolean overlaps(Rect other) {
      return x < other.x + other.width
          && x + width > other.x
          && y < other.y + other.height
          && y + height > other.y;
    }
  }

  public record Layout(Map<String, Rect> panels, double scale, boolean fits) {}

  public static Layout fit(List<Panel> panels, int width, int height, double requested) {
    requested = Double.isFinite(requested) ? Math.clamp(requested, .5, 2) : 1;
    if (panels.isEmpty()) return new Layout(Map.of(), requested, true);
    if (width < 40 || height < 60) return new Layout(Map.of(), .5, false);
    int maxColumns = Math.clamp((width - 16) / 90, 1, 4), bottom = height - 36;
    var sorted = new ArrayList<>(panels);
    sorted.sort(
        Comparator.<Panel>comparingInt(p -> p.id().equals("enabled_mods") ? 0 : 1)
            .thenComparing(Comparator.comparingInt(Panel::height).reversed()));
    for (double scale = requested; scale >= .499; scale -= .05) {
      for (int columns = 1; columns <= maxColumns; columns++) {
        int cell = (width - 16 - (columns - 1) * 6) / columns;
        int[] y = new int[columns];
        Arrays.fill(y, 8);
        var result = new LinkedHashMap<String, Rect>();
        boolean fits = true;
        for (Panel panel : sorted) {
          int column = 0;
          for (int i = 1; i < columns; i++) if (y[i] < y[column]) column = i;
          int w = Math.max(12, (int) Math.ceil(panel.width * scale));
          if (w > cell) {
            fits = false;
            break;
          }
          int h = Math.max(10, (int) Math.ceil(panel.height * scale));
          if (y[column] + h > bottom) {
            fits = false;
            break;
          }
          result.put(panel.id, new Rect(8 + column * (cell + 6), y[column], w, h));
          y[column] += h + 4;
        }
        if (fits) return new Layout(Collections.unmodifiableMap(result), scale, true);
      }
    }
    return new Layout(Map.of(), .5, false);
  }
}
