package dev.justeck.rusty.core;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;

public final class AtomicSettings {
  private AtomicSettings() {}

  public static void write(Path destination, String content) throws IOException {
    Path parent = destination.toAbsolutePath().getParent();
    Files.createDirectories(parent);
    Path temporary = Files.createTempFile(parent, "rusty-", ".tmp");
    try {
      Files.writeString(temporary, content, StandardCharsets.UTF_8);
      for (int attempt = 0; ; attempt++) {
        try {
          try {
            Files.move(
                temporary,
                destination,
                StandardCopyOption.ATOMIC_MOVE,
                StandardCopyOption.REPLACE_EXISTING);
          } catch (AtomicMoveNotSupportedException e) {
            Files.move(temporary, destination, StandardCopyOption.REPLACE_EXISTING);
          }
          return;
        } catch (AccessDeniedException e) {
          if (attempt >= 3) throw e;
          try {
            Thread.sleep(10L * (attempt + 1));
          } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            throw new IOException("Interrupted saving settings", interrupted);
          }
        }
      }
    } finally {
      Files.deleteIfExists(temporary);
    }
  }
}
