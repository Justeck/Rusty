package dev.justeck.rusty;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.mojang.blaze3d.platform.InputConstants;
import dev.justeck.rusty.core.*;
import java.nio.file.*;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/** Main-thread lifecycle owner. Meshing consumes only the immutable View snapshot. */
public final class RustyClient {
  public static final Logger LOG = LoggerFactory.getLogger("Rusty");
  public static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();
  public static final SessionPolicy POLICY = new SessionPolicy();
  public static Settings config = new Settings();
  public static volatile View view = View.empty();
  private static final Map<String, Activation> INPUT = new HashMap<>();
  private static final Activation MENU = new Activation();
  private static final Map<Integer, Integer> KEY_EVENTS = new HashMap<>();

  public static void keyPressed(int key) {
    var mc = Minecraft.getInstance();
    if (mc.player != null
        && mc.level != null
        && mc.gui.screen() == null
        && mc.isWindowActive()
        && key >= 0
        && key < 512) KEY_EVENTS.merge(key, 1, (a, b) -> Math.min(16, a + b));
  }

  private static int presses(String id) {
    return KEY_EVENTS.getOrDefault(config.bindings.getOrDefault(id, -1), 0);
  }

  private static ClientLevel level;
  private static Object connection;
  private static Object pendingPermissionSender;
  private static int pendingPermissionMask;
  private static String token = "";
  private static boolean initialized, previousXray;
  private static volatile Set<String> effective = Set.of();
  public static volatile boolean zoomHeld;
  private static int terrainHash;
  private static long saveAfter;
  private static int tick;
  public static Map<Long, String> targets = Map.of();
  public static boolean partial;
  public static String notice = "";
  public static long lastFrameNanos;
  public static double speed;
  private static Vec3 lastBody;
  public static final ArrayDeque<Vec3> trail = new ArrayDeque<>();

  public static record View(
      boolean xray, boolean context, double edge, int minY, int maxY, Set<String> targets) {
    public static View empty() {
      return new View(false, true, .067, -64, 319, Set.of());
    }

    public boolean selected(BlockState state, BlockPos pos) {
      return pos.getY() >= minY
          && pos.getY() <= maxY
          && targets.contains(BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString());
    }
  }

  private RustyClient() {}

  public static void init() {
    if (initialized) return;
    initialized = true;
    var path = configPath();
    try {
      if (Files.exists(path)) {
        if (Files.size(path) > 1024 * 1024)
          throw new IllegalArgumentException("Settings file exceeds 1 MiB");
        var loaded = GSON.fromJson(Files.readString(path), Settings.class);
        if (loaded == null) throw new IllegalArgumentException("Empty settings");
        loaded.validate();
        config = loaded;
      }
    } catch (Exception e) {
      LOG.warn("Settings retained on disk; using safe defaults", e);
      notice = "Settings could not be read; defaults loaded.";
    }
    config.bindings.putIfAbsent("menu", InputConstants.KEY_RSHIFT);
    config.bindings.putIfAbsent("xray", InputConstants.KEY_X);
    config.bindings.putIfAbsent("zoom", InputConstants.KEY_C);
    publish();
    terrainHash = terrainHash();
    LOG.info(
        "Rusty Java 1.3.0-beta.2 initialized ({})", System.getProperty("rusty.edition", "loader"));
  }

  public static Path configPath() {
    return Minecraft.getInstance().gameDirectory.toPath().resolve("config/rusty.json");
  }

  public static void save() {
    config.validate();
    INPUT.forEach((id, input) -> input.set(requested(id)));
    var path = configPath();
    try {
      AtomicSettings.write(path, GSON.toJson(config));
    } catch (Exception e) {
      LOG.error("Could not save settings", e);
      notice = "Settings were not saved; check the game log.";
    }
    publish();
    if (terrainHash() != terrainHash) refresh();
  }

  public static void changed() {
    saveAfter = System.nanoTime() + 250_000_000L;
  }

  public static void shutdown() {
    if (initialized) save();
  }

  public static void menuClosed() {
    MENU.sample(down("menu"), false, false);
    MENU.set(false);
  }

  private static void publish() {
    effective =
        config.enabled.stream()
            .filter(POLICY::allows)
            .collect(java.util.stream.Collectors.toUnmodifiableSet());
  }

  private static int terrainHash() {
    return Objects.hash(
        active("xray"),
        active("finder"),
        config.context,
        config.textureVisibility,
        config.minY,
        config.maxY,
        config.scanRadius,
        config.maxTargets,
        config.targets);
  }

  public static boolean requested(String id) {
    return config.enabled.contains(id);
  }

  public static boolean active(String id) {
    return effective.contains(id);
  }

  public static boolean down(String id) {
    int code = config.bindings.getOrDefault(id, -1);
    return code >= 0 && InputConstants.isKeyDown(code);
  }

  public static void set(String id, boolean enabled) {
    if (enabled && !requested(id)) RustyAssistance.rearm(id);
    if (enabled) config.enabled.add(id);
    else config.enabled.remove(id);
    INPUT.computeIfAbsent(id, k -> new Activation()).set(enabled);
    if (enabled && !POLICY.allows(id))
      notice = "Requires this server's permission: " + SessionPolicy.required(id);
    save();
  }

  public static void refresh() {
    publish();
    terrainHash = terrainHash();
    var mc = Minecraft.getInstance();
    view =
        new View(
            active("xray") && config.textureVisibility < 100,
            config.context,
            Settings.textureEdge(config.textureVisibility),
            config.minY,
            config.maxY,
            Set.copyOf(config.targets));
    RustyScanner.clear();
    targets = Map.of();
    partial = false;
    if (mc.level != null) mc.levelExtractor.allChanged();
  }

  public static String scope() {
    return token + "/" + (level == null ? "none" : level.dimension().identifier());
  }

  /** Receiver must pass the listener which actually delivered the server message. */
  public static void permissions(Object sender, int mask) {
    if (sender != Minecraft.getInstance().getConnection()) return;
    if (sender != connection) {
      pendingPermissionSender = sender;
      pendingPermissionMask = mask;
      return;
    }
    var grants = new HashSet<String>();
    String[] keys = {"view", "combat", "movement", "assistance"};
    for (int i = 0; i < keys.length; i++) if ((mask & (1 << i)) != 0) grants.add(keys[i]);
    if (POLICY.grant(token, grants)) {
      notice = mask == 0 ? "Server permissions revoked." : "Server permissions received.";
      refresh();
    }
  }

  public static void tick() {
    init();
    var mc = Minecraft.getInstance();
    tick++;
    if (connection != mc.getConnection()) {
      KEY_EVENTS.clear();
      connection = mc.getConnection();
      POLICY.disconnect();
      token = UUID.randomUUID().toString();
      if (connection != null) POLICY.connect(token, mc.hasSingleplayerServer());
      INPUT.forEach((id, input) -> input.contextChanged(requested(id), down(id)));
      previousXray = false;
      RustyCamera.reset();
      RustyHud.reset();
      RustyAssistance.reset();
    }
    if (pendingPermissionSender != null) {
      Object sender = pendingPermissionSender;
      int mask = pendingPermissionMask;
      pendingPermissionSender = null;
      if (sender == connection && sender == mc.getConnection()) permissions(sender, mask);
    }
    if (level != mc.level) {
      KEY_EVENTS.clear();
      level = mc.level;
      lastBody = null;
      trail.clear();
      RustyNavigation.reset();
      RustyAssistance.reset();
      RustyCamera.reset();
      RustyHud.reset();
      refresh();
    }
    boolean focused = mc.isWindowActive() && mc.gui.screen() == null;
    zoomHeld = focused && down("zoom");
    boolean menu = MENU.sampleQueued(down("menu"), false, mc.isWindowActive(), presses("menu"));
    if (menu && !(mc.gui.screen() instanceof RustyScreen)) {
      mc.gui.setScreen(new RustyScreen());
      MENU.set(false);
    }
    for (var module : Catalog.ALL) {
      if (module.id().equals("zoom")) continue;
      var input =
          INPUT.computeIfAbsent(
              module.id(),
              k -> {
                var a = new Activation();
                a.set(requested(k));
                return a;
              });
      boolean next =
          input.sampleQueued(
              down(module.id()),
              module.id().equals("xray") && config.holdXray,
              focused,
              presses(module.id()));
      if (next != requested(module.id())) {
        if (next) {
          config.enabled.add(module.id());
          RustyAssistance.rearm(module.id());
        } else config.enabled.remove(module.id());
        if (module.id().equals("xray")) refresh();
      }
    }
    KEY_EVENTS.clear();
    publish();
    if (saveAfter != 0 && System.nanoTime() >= saveAfter) {
      saveAfter = 0;
      save();
    }
    if (mc.player == null || level == null) return;
    boolean xray = active("xray") && config.textureVisibility < 100;
    if (xray != previousXray) {
      previousXray = xray;
      refresh();
    }
    if (lastBody != null) {
      double distance = mc.player.position().distanceTo(lastBody);
      speed = distance < 16 ? distance * 20 : 0;
      if (active("breadcrumbs")
          && distance < 16
          && (trail.isEmpty() || trail.getLast().distanceToSqr(mc.player.position()) >= 4)) {
        trail.addLast(mc.player.position());
        while (trail.size() > 512) trail.removeFirst();
      }
    }
    lastBody = mc.player.position();
    if (active("xray") || active("finder")) RustyScanner.tick();
    else {
      targets = Map.of();
      partial = false;
    }
    RustyNavigation.tick(focused);
    RustyAssistance.tick(focused);
    RustyCamera.tick();
    RustyPlanner.tick(focused);
    RustyOverlay.capture();
    RustyHud.capture();
  }

  public static void exportProfile(String name) {
    if (!name.matches("[a-zA-Z0-9_-]{1,48}")) return;
    try {
      Path dir = configPath().getParent().resolve("rusty-profiles");
      Files.createDirectories(dir);
      Files.writeString(dir.resolve(name + ".json"), GSON.toJson(config));
      notice = "Saved profile " + name;
    } catch (Exception e) {
      notice = "Profile save failed.";
      LOG.warn(notice, e);
    }
  }

  public static void importProfile(String name) {
    if (!name.matches("[a-zA-Z0-9_-]{1,48}")) return;
    try {
      Path path = configPath().getParent().resolve("rusty-profiles/" + name + ".json");
      if (Files.size(path) > 1024 * 1024)
        throw new IllegalArgumentException("Profile exceeds 1 MiB");
      Settings next = GSON.fromJson(Files.readString(path), Settings.class);
      next.validate();
      next.bindings = config.bindings;
      next.overlays = config.overlays;
      next.theme = config.theme;
      next.menuScale = config.menuScale;
      next.favorites = config.favorites;
      next.recent = config.recent;
      next.hudScale = config.hudScale;
      next.overlayScales = config.overlayScales;
      next.reducedMotion = config.reducedMotion;
      config = next;
      INPUT.clear();
      save();
      notice = "Loaded profile " + name;
    } catch (Exception e) {
      notice = "Profile load failed; current settings retained.";
      LOG.warn(notice, e);
    }
  }
}
