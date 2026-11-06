package dev.justeck.rusty.core;

import static org.junit.jupiter.api.Assertions.*;

import java.nio.file.*;
import java.util.concurrent.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class SchematicCommitTest {
  @TempDir Path directory;

  @Test
  void cancelAndScopeBeforeClaimPreserveNewAndReplacementDestinations() throws Exception {
    for (boolean existing : new boolean[] {false, true})
      for (boolean explicitCancel : new boolean[] {false, true}) {
        Path destination = directory.resolve("chosen.schem");
        Files.deleteIfExists(destination);
        if (existing) Files.writeString(destination, "original");
        var permit = new SchematicCommit();
        var prepared = new CountDownLatch(1);
        var release = new CountDownLatch(1);
        var current = new java.util.concurrent.atomic.AtomicBoolean(true);
        var error = new java.util.concurrent.atomic.AtomicReference<Throwable>();
        var worker =
            Thread.ofPlatform()
                .start(() -> {
                  try (var save = new SchematicSave(destination, existing)) {
                    save.write("chosen".getBytes());
                    save.checkDestination();
                    prepared.countDown();
                    assertTrue(release.await(5, TimeUnit.SECONDS));
                    assertFalse(permit.claim(current::get));
                    assertThrows(IllegalStateException.class, () -> save.commit(permit));
                  } catch (Throwable e) {
                    error.set(e);
                  }
                });
        assertTrue(prepared.await(5, TimeUnit.SECONDS));
        if (explicitCancel) assertTrue(permit.cancel());
        else current.set(false);
        release.countDown();
        worker.join(5000);
        assertFalse(worker.isAlive());
        assertNull(error.get());
        assertEquals(existing, Files.exists(destination));
        if (existing) assertEquals("original", Files.readString(destination));
        try (var paths = Files.list(directory)) {
          assertEquals(existing ? 1 : 0, paths.count(), "Cancelled temporary file removed");
        }
      }
  }

  @Test
  void blockedAfterClaimCannotBlockCancelAndChosenSnapshotMayFinish() throws Exception {
    for (boolean existing : new boolean[] {false, true}) {
      var destination = directory.resolve(existing + ".schem");
      if (existing) Files.writeString(destination, "original");
      var permit = new SchematicCommit();
      var claimed = new CountDownLatch(1);
      var release = new CountDownLatch(1);
      var error = new java.util.concurrent.atomic.AtomicReference<Throwable>();
      Thread worker =
          Thread.ofPlatform()
              .start(() -> {
                try (var save = new SchematicSave(destination, existing)) {
                  save.write("chosen".getBytes());
                  save.checkDestination();
                  assertTrue(permit.claim(() -> true));
                  claimed.countDown();
                  assertTrue(release.await(5, TimeUnit.SECONDS));
                  save.commit(permit);
                } catch (Throwable e) {
                  error.set(e);
                }
              });
      assertTrue(claimed.await(5, TimeUnit.SECONDS));
      try {
        assertTimeoutPreemptively(
            java.time.Duration.ofMillis(250), () -> assertFalse(permit.cancel()));
        assertTrue(permit.committing(), "Truthful may-finish boundary remains available");
      } finally {
        release.countDown();
      }
      worker.join(5000);
      assertFalse(worker.isAlive());
      assertNull(error.get());
      assertEquals("chosen", Files.readString(destination));
    }
  }

  @Test
  void destinationChangesDuringPreparationArePreservedAndTempIsRemoved() throws Exception {
    var path = directory.resolve("changed.schem");
    Files.writeString(path, "original");
    try (var save = new SchematicSave(path, true)) {
      Files.writeString(path, "another owner's new contents");
      save.write("snapshot".getBytes());
      assertThrows(java.io.IOException.class, save::checkDestination);
    }
    assertEquals("another owner's new contents", Files.readString(path));
    assertThrows(java.io.IOException.class, () -> new SchematicSave(path, false));
    try (var paths = Files.list(directory)) {
      assertEquals(1, paths.count());
    }
  }
}
