package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import java.util.*;
import org.junit.jupiter.api.Test;

class SchematicSurveyTest {
  @Test
  void alreadyCountedCellsCannotCompleteAfterMutationAndStableRetryCan() {
    var watch =
        new SchematicSurvey(List.of(new Schematic.Pos(0, 1, 0), new Schematic.Pos(17, 1, 0)));
    var pass = new SchematicSurvey.Pass();
    var counts = new SchematicSurvey.Counts();
    pass.begin(watch.revision());
    for (int i = 0; i < 2048; i++) counts.observe(1, false, false, false);
    watch.column(0, 0);
    assertFalse(pass.coherent(watch.revision()));
    assertTrue(pass.begin(watch.revision()));
    counts = new SchematicSurvey.Counts();
    counts.observe(2, false, false, false);
    for (int i = 1; i < 4096; i++) counts.observe(1, false, false, false);
    assertEquals(4095, counts.matching);
    assertEquals(1, counts.missing);
    assertTrue(pass.coherent(watch.revision()));
    watch.column(0, 0);
    assertTrue(pass.begin(watch.revision()));
    counts = new SchematicSurvey.Counts();
    for (int i = 0; i < 4096; i++) counts.observe(1, false, false, false);
    assertEquals(4096, counts.matching);
    assertTrue(pass.coherent(watch.revision()));
  }

  @Test
  void distantTrafficDoesNotStarveAndContinuousTargetChangesAreBounded() {
    var watch = new SchematicSurvey(List.of(new Schematic.Pos(0, 1, 0)));
    var pass = new SchematicSurvey.Pass();
    pass.begin(watch.revision());
    for (int i = 0; i < 2000; i++) watch.column(80, 80);
    assertTrue(pass.coherent(watch.revision()));
    for (int i = 0; i < 9; i++) {
      watch.all();
      pass.begin(watch.revision());
    }
    assertTrue(pass.exhausted);
    assertFalse(pass.coherent(watch.revision()));
  }

  @Test
  void mixedReadbackKeepsCategoriesAndNewCellsSeparate() {
    var c = new SchematicSurvey.Counts();
    c.observe(1, false, false, false); // preexisting
    c.observe(1, false, false, true); // one received door action, two completed cells
    c.observe(1, false, false, true);
    c.observe(3, false, false, false); // occupied mismatch
    c.observe(2, false, false, false); // material absent
    c.observe(2, true, false, false); // manual state
    c.observe(2, false, true, false); // route exhausted
    c.observe(0, false, false, false); // unsettled/unavailable
    assertEquals(8, c.scanned);
    assertEquals(3, c.matching);
    assertEquals(2, c.completed);
    assertEquals(1, c.occupied);
    assertEquals(3, c.missing);
    assertEquals(1, c.manual);
    assertEquals(1, c.inaccessible);
    assertEquals(1, c.unknown);
  }
}
