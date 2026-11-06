package dev.justeck.rusty.core;

import java.nio.file.*;

public final class SchematicFixtureProbe {
  public static void main(String[] args) throws Exception {
    for (String file : args) {
      var d = Schematic.read(Files.readAllBytes(Path.of(file)));
      long cells = d.cells().stream().filter(c -> !c.state().skipped()).count();
      long items = d.cells().stream().mapToLong(c -> Schematic.materialUnits(c.state())).sum();
      if (cells != 12 || items != 10)
        throw new AssertionError(file + ": " + cells + " cells / " + items + " items");
      System.out.println(Path.of(file).getFileName() + ":12 cells/10 items; " + d.format());
    }
  }
}
