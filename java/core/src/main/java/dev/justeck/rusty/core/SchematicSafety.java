package dev.justeck.rusty.core;

import java.util.Objects;

/** Receipt-based transfers: client inventory prediction never calls received. */
public final class SchematicSafety {
  public static boolean coupledCovered(
      Schematic.Cell secondary,
      java.util.function.Function<Schematic.Pos, Schematic.State> planned) {
    var state = secondary.state();
    var props = new java.util.TreeMap<>(state.properties());
    var p = secondary.pos();
    Schematic.Pos primary;
    if ("upper".equals(props.get("half"))) {
      props.put("half", "lower");
      if (p.y() == Integer.MIN_VALUE) return false;
      primary = new Schematic.Pos(p.x(), p.y() - 1, p.z());
    } else if ("head".equals(props.get("part"))) {
      props.put("part", "foot");
      int dx = 0, dz = 0;
      switch (props.getOrDefault("facing", "")) {
        case "east" -> dx = -1;
        case "west" -> dx = 1;
        case "south" -> dz = -1;
        case "north" -> dz = 1;
        default -> {
          return false;
        }
      }
      try {
        primary = new Schematic.Pos(Math.addExact(p.x(), dx), p.y(), Math.addExact(p.z(), dz));
      } catch (ArithmeticException e) {
        return false;
      }
    } else return false;
    return new Schematic.State(state.id(), props).equals(planned.apply(primary));
  }

  public static final class Transfer<T> {
    private final int source, destination;
    private final T incoming, displaced;
    private boolean a, b, rejected;

    public Transfer(int source, int destination, T incoming, T displaced) {
      this.source = source;
      this.destination = destination;
      this.incoming = incoming;
      this.displaced = displaced;
    }

    public void received(int slot, T value) {
      if (slot == source) {
        a = Objects.equals(displaced, value);
        rejected |= !a;
      }
      if (slot == destination) {
        b = Objects.equals(incoming, value);
        rejected |= !b;
      }
    }

    public boolean matches(T sourceValue, T destinationValue) {
      return !rejected
          && Objects.equals(displaced, sourceValue)
          && Objects.equals(incoming, destinationValue);
    }

    public boolean confirmed() {
      return a && b && !rejected;
    }

    public boolean rejected() {
      return rejected;
    }
  }

  public static boolean progress(Schematic.State wanted, Schematic.State predicted) {
    if (wanted.equals(predicted)) return true;
    if (!wanted.id().equals(predicted.id())) return false;
    var target = new java.util.TreeMap<>(wanted.properties());
    var next = new java.util.TreeMap<>(predicted.properties());
    if (wanted.id().endsWith("_slab")
        && "double".equals(target.get("type"))
        && java.util.Set.of("bottom", "top").contains(next.getOrDefault("type", ""))) {
      target.remove("type");
      next.remove("type");
      return target.equals(next);
    }
    for (String key : new String[] {"layers", "candles", "eggs", "pickles"})
      if (target.containsKey(key) && next.containsKey(key)) {
        int a, b;
        try {
          a = Integer.parseInt(target.remove(key));
          b = Integer.parseInt(next.remove(key));
        } catch (NumberFormatException e) {
          return false;
        }
        int max = key.equals("layers") ? 8 : 4;
        return b >= 1 && b < a && a <= max && target.equals(next);
      }
    return false;
  }

  public static String stopReason(
      boolean manual,
      boolean permission,
      boolean focused,
      float health,
      float previous,
      int food,
      int ticks) {
    return stopReason(manual, permission, focused, health, previous, food, ticks, 12, 10);
  }

  public static String stopReason(
      boolean manual,
      boolean permission,
      boolean focused,
      float health,
      float previous,
      int food,
      int ticks,
      int healthLimit,
      int foodLimit) {
    if (manual) return "Manual input";
    if (!permission) return "Permission unavailable";
    if (!focused) return "Menu or focus changed";
    if (health < healthLimit) return "Health below " + healthLimit + "/20";
    if (health < previous) return "Damage received";
    if (food < foodLimit) return "Food below " + foodLimit + "/20";
    if (ticks > 36000) return "30-minute deadline";
    return null;
  }
}
