package dev.justeck.rusty;

import java.awt.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.TimeUnit;
import javax.swing.*;

/** Native FileDialog runs in an isolated desktop helper because Minecraft forces AWT headless. */
public final class SchematicDialogs {
  private static volatile Process helper;
  private static final Object LOCK = new Object();

  public record Choice(Path path, boolean overwrite) {}

  public static void cancel() {
    synchronized (LOCK) {
      var p = helper;
      if (p != null) p.destroy();
    }
  }

  public static Choice choose(boolean save, java.util.function.BooleanSupplier current)
      throws Exception {
    String executable =
        System.getProperty("os.name").toLowerCase(Locale.ROOT).contains("win")
            ? "java.exe"
            : "java";
    Path helperJar = Files.createTempFile("rusty-native-dialog-", ".jar");
    try {
      try (var out = new java.util.jar.JarOutputStream(Files.newOutputStream(helperJar))) {
        for (String resource :
            new String[] {
              "dev/justeck/rusty/SchematicDialogs.class",
              "dev/justeck/rusty/SchematicDialogs$Choice.class"
            }) {
          out.putNextEntry(new java.util.jar.JarEntry(resource));
          try (var input = SchematicDialogs.class.getClassLoader().getResourceAsStream(resource)) {
            if (input == null) throw new java.io.IOException("Native chooser helper missing");
            input.transferTo(out);
          }
          out.closeEntry();
        }
      }
      return chooseHelper(helperJar, executable, save, current);
    } finally {
      try {
        Files.deleteIfExists(helperJar);
      } catch (java.io.IOException e) {
        helperJar.toFile().deleteOnExit();
      }
    }
  }

  private static Choice chooseHelper(
      Path helperJar, String executable, boolean save, java.util.function.BooleanSupplier current)
      throws Exception {
    Process p;
    synchronized (LOCK) {
      if (!current.getAsBoolean()) return null;
      p =
          new ProcessBuilder(
                  Path.of(System.getProperty("java.home"), "bin", executable).toString(),
                  "-Djava.awt.headless=false",
                  "-Dfile.encoding=UTF-8",
                  "-cp",
                  helperJar.toString(),
                  SchematicDialogs.class.getName(),
                  save ? "save" : "load")
              .redirectError(ProcessBuilder.Redirect.DISCARD)
              .start();
      helper = p;
    }
    var output = new java.util.concurrent.CompletableFuture<byte[]>();
    Thread.ofVirtual()
        .name("Rusty native dialog output")
        .start(() -> {
          try {
            output.complete(p.getInputStream().readNBytes(65537));
          } catch (Exception e) {
            output.completeExceptionally(e);
          }
        });
    try {
      if (!p.waitFor(30, TimeUnit.MINUTES)) {
        p.destroy();
        throw new java.io.IOException("File chooser timed out");
      }
      byte[] bytes = output.get(5, TimeUnit.SECONDS);
      if (bytes.length > 65536 || p.exitValue() != 0)
        throw new java.io.IOException("Native desktop chooser unavailable");
      String reply = new String(bytes, StandardCharsets.UTF_8).trim();
      if (reply.isEmpty() || reply.equals("CANCEL")) return null;
      boolean overwrite = reply.startsWith("OVERWRITE:");
      String path =
          new String(
              Base64.getDecoder().decode(reply.substring(reply.indexOf(':') + 1)),
              StandardCharsets.UTF_8);
      return new Choice(Path.of(path), overwrite);
    } finally {
      if (helper == p) helper = null;
      p.destroy();
      if (!p.waitFor(5, TimeUnit.SECONDS)) {
        p.destroyForcibly();
        p.waitFor(5, TimeUnit.SECONDS);
      }
    }
  }

  public static void main(String[] args) throws Exception {
    UIManager.setLookAndFeel(UIManager.getSystemLookAndFeelClassName());
    boolean save = args.length > 0 && args[0].equals("save");
    final String[] reply = {"CANCEL"};
    EventQueue.invokeAndWait(() -> {
      var frame = new Frame();
      try {
        frame.setAlwaysOnTop(true);
        var dialog =
            new FileDialog(
                frame,
                save ? "Save Rusty states-only selection" : "Load Rusty schematic (preview only)",
                save ? FileDialog.SAVE : FileDialog.LOAD);
        dialog.setMultipleMode(false);
        dialog.setFile(save ? "rusty-selection.schem" : "*.litematic;*.schem;*.nbt");
        dialog.setFilenameFilter((dir, name) ->
            save
                || name.endsWith(".litematic")
                || name.endsWith(".schem")
                || name.endsWith(".nbt"));
        dialog.setVisible(true);
        if (dialog.getFile() == null) return;
        Path chosen = Path.of(dialog.getDirectory(), dialog.getFile());
        if (save && !chosen.getFileName().toString().toLowerCase(Locale.ROOT).endsWith(".schem"))
          chosen = chosen.resolveSibling(chosen.getFileName() + ".schem");
        boolean overwrite = save && Files.exists(chosen);
        if (overwrite
            && JOptionPane.showConfirmDialog(
                    frame,
                    "Replace " + chosen.getFileName() + "?",
                    "Confirm schematic replacement",
                    JOptionPane.YES_NO_OPTION)
                != JOptionPane.YES_OPTION) return;
        reply[0] =
            (overwrite ? "OVERWRITE:" : "FILE:")
                + Base64.getEncoder()
                    .encodeToString(chosen.toString().getBytes(StandardCharsets.UTF_8));
      } finally {
        frame.dispose();
      }
    });
    if (reply[0].length() > 65536) System.exit(2);
    System.out.print(reply[0]);
    System.exit(0);
  }
}
