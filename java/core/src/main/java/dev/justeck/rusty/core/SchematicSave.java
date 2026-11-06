package dev.justeck.rusty.core;

import java.io.IOException;
import java.nio.file.*;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.Objects;

/** Prepared, bounded bytes remain temporary until the request wins its commit claim. */
public final class SchematicSave implements AutoCloseable {
  private final Path destination, temporary;
  private final BasicFileAttributes before;
  private boolean committed;

  public SchematicSave(Path destination, boolean overwrite) throws IOException {
    this.destination = destination;
    before =
        Files.exists(destination)
            ? Files.readAttributes(destination, BasicFileAttributes.class)
            : null;
    if (before != null && !overwrite)
      throw new IOException("Existing file preserved; choose another name");
    temporary = Files.createTempFile(destination.getParent(), ".rusty-states-", ".tmp");
  }

  public void write(byte[] bytes) throws IOException {
    Files.write(temporary, bytes);
  }

  public void checkDestination() throws IOException {
    if (before == null) {
      if (Files.exists(destination)) throw new IOException("Destination now exists; preserved");
    } else {
      var now = Files.readAttributes(destination, BasicFileAttributes.class);
      if (!Objects.equals(before.fileKey(), now.fileKey())
          || before.size() != now.size()
          || !before.lastModifiedTime().equals(now.lastModifiedTime()))
        throw new IOException("Destination changed after confirmation");
    }
  }

  /** Caller must have won the per-request claim immediately before this final operation. */
  public void commit(SchematicCommit permit) throws IOException {
    if (!permit.committing()) throw new IllegalStateException("No commit ownership");
    if (before != null)
      Files.move(
          temporary,
          destination,
          StandardCopyOption.ATOMIC_MOVE,
          StandardCopyOption.REPLACE_EXISTING);
    else {
      Files.createLink(destination, temporary);
      Files.delete(temporary);
    }
    committed = true;
  }

  public void close() throws IOException {
    if (!committed) Files.deleteIfExists(temporary);
  }
}
