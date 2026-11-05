package dev.justeck.rusty.qa;

import dev.justeck.rusty.*;
import dev.justeck.rusty.core.*;
import java.nio.file.*;
import java.time.Instant;
import java.util.*;
import java.util.function.BooleanSupplier;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.Vec3;

/**
 * Explicit development-only fixture entry. Refuses personal directories/worlds and non-loopback
 * servers.
 */
public final class GameAcceptance {
  private static volatile boolean menuCaptureComplete;
  private static final String MODE = System.getProperty("rusty.qa", "");
  private static final List<Map<String, Object>> checks = new ArrayList<>();
  private static int ticks, stage, wait;
  private static boolean finished, prepared;
  private static Vec3 body, camera;
  private static float bodyYaw;
  private static net.minecraft.world.entity.LivingEntity combatTarget;
  private static float combatHealth;
  private static final List<Map<String, Object>> performance = new ArrayList<>();
  private static final List<Step> steps = new ArrayList<>();

  private record Step(String name, Runnable action, BooleanSupplier condition, int minimumTicks) {}

  private static void add(String name, Runnable action, BooleanSupplier condition, int ticks) {
    steps.add(new Step(name, action, condition, ticks));
  }

  private static void check(String name, boolean value) {
    if (!value) throw new AssertionError(name);
    checks.add(Map.of("name", name, "passed", true));
  }

  private static void command(String command) {
    var mc = Minecraft.getInstance();
    var server = mc.getSingleplayerServer();
    server.execute(() ->
        server
            .getCommands()
            .performPrefixedCommand(
                server.createCommandSourceStack().withSuppressedOutput(), command));
  }

  public static void tick() {
    if (MODE.isEmpty() || finished) return;
    var mc = Minecraft.getInstance();
    ticks++;
    try {
      String directory =
          mc.gameDirectory.toPath().toAbsolutePath().normalize().toString().replace('\\', '/');
      if (!directory.endsWith("/java/fabric/run") && !directory.endsWith("/java/neoforge/run"))
        throw new IllegalStateException("QA refuses a non-development game directory");
      if (mc.player == null || mc.level == null) {
        if (ticks > 2400) throw new AssertionError("Timed out joining owned fixture");
        return;
      }
      if (!prepared) {
        if (MODE.equals("singleplayer")) {
          if (!mc.hasSingleplayerServer())
            throw new IllegalStateException("QA refuses a remote world in singleplayer mode");
          Path world =
              mc.getSingleplayerServer()
                  .getWorldPath(net.minecraft.world.level.storage.LevelResource.ROOT)
                  .toRealPath();
          Path owned = mc.gameDirectory.toPath().resolve("saves/RustyLab").toRealPath();
          if (!world.equals(owned))
            throw new IllegalStateException(
                "QA refuses any world other than the exact owned RustyLab directory");
          prepareLocal();
        } else {
          if (mc.getCurrentServer() == null || !mc.getCurrentServer().ip.startsWith("127.0.0.1:"))
            throw new IllegalStateException("QA remote fixture must be loopback");
          prepareRemote();
        }
        prepared = true;
        wait = 0;
      }
      if (stage >= steps.size()) {
        finish(true, null);
        return;
      }
      Step current = steps.get(stage);
      if (ticks % 20 == 0 && mc.player != null)
        performance.add(
            Map.of(
                "tick",
                ticks,
                "fps",
                mc.getFps(),
                "heap_used_bytes",
                Runtime.getRuntime().totalMemory() - Runtime.getRuntime().freeMemory(),
                "targets",
                RustyClient.targets.size(),
                "overlay_primitives",
                RustyOverlay.renderedGeometry));
      if (wait == 0) {
        RustyClient.LOG.info("Rusty QA step {}: {}", stage, current.name);
        current.action.run();
      }
      wait++;
      if (wait >= current.minimumTicks && current.condition.getAsBoolean()) {
        check(current.name, true);
        stage++;
        wait = 0;
        write(false, null);
      } else if (wait > 600) throw new AssertionError("Timed out: " + current.name);
    } catch (Throwable e) {
      RustyClient.LOG.error("Rusty game fixture failed", e);
      finish(false, e.toString());
    }
  }

  private static void prepareLocal() {
    var mc = Minecraft.getInstance();
    mc.getTutorial().setStep(net.minecraft.client.tutorial.TutorialSteps.NONE);
    RustyClient.config = new Settings();
    RustyClient.config.bindings.put("menu", com.mojang.blaze3d.platform.InputConstants.KEY_RSHIFT);
    RustyClient.config.bindings.put("xray", com.mojang.blaze3d.platform.InputConstants.KEY_X);
    RustyClient.config.bindings.put("zoom", com.mojang.blaze3d.platform.InputConstants.KEY_C);
    RustyClient.save();
    addCodecChecks();
    add(
        "Fixture setup is received from owned integrated server",
        () -> {
          command("fill -20 98 -20 20 104 20 air");
          command("fill -20 98 -20 20 99 20 stone");
          command("setblock 0 99 3 diamond_ore");
          command("setblock 2 99 3 deepslate_gold_ore");
          command("setblock 0 97 5 emerald_ore");
          command("setblock 4 100 3 chest");
          command("tp @a 0.5 100 0.5 0 25");
          command("gamemode survival @a");
          command("time set day");
          command("weather clear");
        },
        () ->
            mc.level.getBlockState(new BlockPos(0, 99, 3)).is(Blocks.DIAMOND_ORE)
                && mc.player.getY() > 99,
        40);
    add(
        "Singleplayer capabilities are granted locally",
        () -> {},
        () -> RustyClient.POLICY.allows("xray") && RustyClient.POLICY.allows("trigger_bot"),
        2);
    add(
        "Minecraft module screen constructs its real widgets",
        () -> mc.gui.setScreen(new RustyScreen()),
        () -> mc.gui.screen() instanceof RustyScreen s && s.children().size() > 12,
        10);
    add(
        "Menu closes and normal gameplay resumes",
        () -> mc.gui.setScreen(null),
        () -> mc.gui.screen() == null,
        5);
    add(
        "Xray finds exposed and buried authoritative received ores",
        () -> {
          RustyClient.config.scanRadius = 16;
          RustyClient.config.minY = 96;
          RustyClient.config.maxY = 104;
          RustyClient.config.targets = Settings.defaultTargets();
          RustyClient.set("xray", true);
          TextureFrames.framedQuads.set(0);
        },
        () ->
            RustyClient.targets.containsKey(BlockPos.asLong(0, 99, 3))
                && RustyClient.targets.containsKey(BlockPos.asLong(2, 99, 3))
                && RustyClient.targets.containsKey(BlockPos.asLong(0, 97, 5))
                && TextureFrames.framedQuads.get() > 0,
        20);
    add(
        "Xray alone does not render saved Finder lines or boxes",
        () -> {
          RustyClient.config.tracers = true;
          RustyClient.config.outlines = true;
          RustyClient.set("finder", false);
        },
        () -> RustyClient.active("xray") && !RustyClient.active("finder")
            && RustyClient.targets.size() >= 3 && RustyOverlay.renderedGeometry == 0,
        10);
    add(
        "Finder explicitly enables target geometry alongside Xray",
        () -> RustyClient.set("finder", true),
        () -> RustyClient.active("xray") && RustyClient.active("finder")
            && RustyOverlay.renderedGeometry > 0,
        10);
    add(
        "Disabling Xray preserves independently enabled Finder geometry",
        () -> RustyClient.set("xray", false),
        () -> !RustyClient.active("xray") && RustyClient.active("finder")
            && RustyOverlay.renderedGeometry > 0,
        10);
    add(
        "Disabling Finder restores Xray-only rendering without changing line preferences",
        () -> {
          RustyClient.set("finder", false);
          RustyClient.set("xray", true);
        },
        () -> RustyClient.active("xray") && !RustyClient.active("finder")
            && RustyClient.config.tracers && RustyClient.config.outlines
            && RustyOverlay.renderedGeometry == 0,
        10);
    add(
        "Received block removal invalidates the target cache",
        () -> command("setblock 0 99 3 stone"),
        () ->
            !RustyClient.targets.containsKey(BlockPos.asLong(0, 99, 3))
                && !mc.level.getBlockState(new BlockPos(0, 99, 3)).is(Blocks.DIAMOND_ORE),
        10);
    add(
        "Received block restoration is discovered again",
        () -> command("setblock 0 99 3 diamond_ore"),
        () -> RustyClient.targets.containsKey(BlockPos.asLong(0, 99, 3)),
        20);
    add(
        "Maximum texture context restores ordinary terrain rendering",
        () -> {
          RustyClient.config.textureVisibility = 100;
          RustyClient.save();
        },
        () -> !RustyClient.view.xray(),
        10);
    add(
        "Minimum texture context keeps thin original-texture edges",
        () -> {
          RustyClient.config.textureVisibility = 0;
          RustyClient.save();
        },
        () ->
            RustyClient.view.xray() && RustyClient.view.edge() > 0 && RustyClient.view.edge() < .01,
        10);
    add(
        "Default texture coverage and custom received block selection",
        () -> {
          RustyClient.config.textureVisibility = 25;
          RustyClient.config.targets.add("minecraft:stone");
          RustyClient.config.maxTargets = 16;
          RustyClient.save();
        },
        () -> RustyClient.partial && RustyClient.targets.size() == 16,
        30);
    add(
        "Toggle off restores view without changing world blocks",
        () -> RustyClient.set("xray", false),
        () ->
            !RustyClient.view.xray()
                && mc.level.getBlockState(new BlockPos(0, 99, 3)).is(Blocks.DIAMOND_ORE),
        10);
    add(
        "Detached camera input moves the view and preserves body position",
        () -> {
          body = mc.player.position();
          bodyYaw = mc.player.getYRot();
          RustyClient.set("freecam", true);
          mc.options.keyUp.setDown(true);
        },
        () ->
            RustyCamera.position.distanceToSqr(body.add(0, mc.player.getEyeHeight(), 0)) > 1
                && mc.player.position().distanceToSqr(body) < .05,
        12);
    add(
        "Settled rendered camera follows detached view without body rotation",
        () -> {
          mc.options.keyUp.setDown(false);
          RustyCamera.turn(100, 40);
        },
        () ->
            Math.abs(mc.player.getYRot() - bodyYaw) < .01
                && Math.abs(RustyCamera.yaw - bodyYaw) > 1
                && mc.gameRenderer.mainCamera().position().distanceToSqr(RustyCamera.position)
                    < .0001,
        4);
    add(
        "Freecam disables cleanly",
        () -> RustyClient.set("freecam", false),
        () -> !RustyClient.active("freecam"),
        5);
    addActionChecks();
    add(
        "Waypoint saves in current stable world/dimension scope",
        RustyNavigation::saveMarker,
        () -> RustyNavigation.selected() != null,
        5);
    add(
        "HUD editor exposes 18 independent panels with footer clearance",
        () -> mc.gui.setScreen(new HudEditor()),
        () ->
            RustyHud.bounds.size() == 18
                && RustyHud.bounds.values().stream()
                    .allMatch(r -> r[1] + r[3] <= mc.getWindow().getGuiScaledHeight() - 60),
        10);
    add(
        "Overlay positions and cleared shortcuts persist to actual local JSON",
        () -> {
          mc.gui.setScreen(null);
          RustyClient.config.overlays.put("coordinates", new double[] {.4, .3});
          RustyClient.config.bindings.put("finder", -1);
          RustyClient.save();
        },
        () -> {
          try {
            var saved =
                RustyClient.GSON.fromJson(
                    Files.readString(RustyClient.configPath()), Settings.class);
            return saved.bindings.get("finder") == -1 && saved.overlays.get("coordinates")[0] == .4;
          } catch (Exception e) {
            return false;
          }
        },
        5);
    add(
        "All module adapters tolerate received world and live rendering",
        () -> {
          RustyClient.config.overlays.clear();
          RustyClient.config.overlayScales.clear();
          RustyClient.config.targets = Settings.defaultTargets();
          RustyClient.config.maxTargets = 1024;
          for (var module : Catalog.ALL)
            if (!Set.of("xray", "freecam", "waypoint_walk", "auto_eat", "trigger_bot", "auto_fish")
                .contains(module.id())) RustyClient.config.enabled.add(module.id());
          RustyClient.save();
          command("summon pig 3 100 3");
        },
        () -> hudFits() && mc.getFps() > 0 && RustyOverlay.renderedGeometry > 0,
        100);
    add(
        "Remote chunk loading removes targets from the former area",
        () -> {
          command("gamemode creative @a");
          command("tp @a 512.5 160 0.5");
        },
        () -> mc.player.getX() > 500 && RustyClient.targets.isEmpty(),
        100);
    add(
        "Returning to received chunks discovers the original ores again",
        () -> command("tp @a 0.5 100 0.5"),
        () -> RustyClient.targets.containsKey(BlockPos.asLong(0, 99, 3)),
        100);
    add(
        "Dimension change resets scanner and scoped markers",
        () -> {
          RustyClient.set("freecam", true);
          command("execute in minecraft:the_nether run tp @a 0.5 160 0.5");
        },
        () ->
            mc.level.dimension().identifier().getPath().equals("the_nether")
                && RustyNavigation.markers().isEmpty()
                && !RustyClient.targets.containsKey(BlockPos.asLong(0, 99, 3))
                && RustyCamera.position.distanceToSqr(new Vec3(.5, 161.62000000476837, .5))
                    < .000001,
        100);
    add(
        "Returning dimension restores permission and target lifecycle",
        () -> {
          RustyClient.set("freecam", false);
          command("execute in minecraft:overworld run tp @a 0.5 100 0.5");
        },
        () ->
            mc.level.dimension().identifier().getPath().equals("overworld")
                && RustyClient.targets.containsKey(BlockPos.asLong(0, 99, 3))
                && RustyClient.POLICY.allows("xray"),
        100);
    add(
        "Profiles round-trip without acquiring multiplayer permission",
        () -> {
          RustyClient.exportProfile("qa");
          RustyClient.importProfile("qa");
        },
        () -> RustyClient.POLICY.allows("xray"),
        5);
    add(
        "Cleared menu binding retains Minecraft Options recovery",
        () -> {
          RustyClient.config.bindings.put("menu", -1);
          RustyClient.save();
          mc.gui.setScreen(
              new net.minecraft.client.gui.screens.options.OptionsScreen(null, mc.options));
        },
        () ->
            mc.gui.screen() != null
                && mc.gui.screen().children().stream()
                    .anyMatch(c ->
                        c instanceof net.minecraft.client.gui.components.Button button
                            && button.getMessage().getString().equals("Rusty")),
        10);
    add(
        "Thirty-second bounded rendering observation completes",
        () -> {
          mc.gui.setScreen(null);
          RustyClient.config = new Settings();
          RustyClient.config.bindings.put(
              "menu", com.mojang.blaze3d.platform.InputConstants.KEY_RSHIFT);
          RustyClient.config.bindings.put("xray", com.mojang.blaze3d.platform.InputConstants.KEY_X);
          RustyClient.config.bindings.put("zoom", com.mojang.blaze3d.platform.InputConstants.KEY_C);
          RustyClient.config.enabled.addAll(Set.of("xray", "finder", "coordinates"));
          RustyClient.save();
        },
        () ->
            RustyClient.targets.size() <= RustyClient.config.maxTargets
                && RustyOverlay.renderedGeometry <= 768
                && mc.getFps() > 0,
        600);
    add(
        "Clean default controls remain ready for visual review",
        () -> {
          RustyClient.set("xray", false);
          RustyClient.set("finder", false);
          RustyClient.set("coordinates", false);
          RustyClient.notice = "";
          command("clear @a");
          command("item replace entity @a weapon.offhand with air");
          command("kill @e[type=minecraft:pig]");
          command("gamemode survival @a");
          command("tp @a 0.5 100 0.5 0 25");
        },
        () -> !RustyClient.view.xray(),
        10);
    add(
        "Final actual menu remains usable after module exercise",
        () -> mc.gui.setScreen(new RustyScreen()),
        () -> mc.gui.screen() instanceof RustyScreen,
        30);
    add(
        "Final menu framebuffer capture completes",
        () ->
            net.minecraft.client.Screenshot.grab(
                mc.gameDirectory,
                "rusty-menu-branding.png",
                mc.gameRenderer.mainRenderTarget(),
                1,
                message -> menuCaptureComplete = true),
        () -> menuCaptureComplete,
        200);
  }

  private static boolean hudFits() {
    var mc = Minecraft.getInstance();
    if (RustyHud.bounds.size() != 18) return false;
    var rectangles = new ArrayList<int[]>(RustyHud.bounds.values());
    for (int i = 0; i < rectangles.size(); i++) {
      var r = rectangles.get(i);
      if (r[0] < 0
          || r[1] < 0
          || r[0] + r[2] > mc.getWindow().getGuiScaledWidth()
          || r[1] + r[3] > mc.getWindow().getGuiScaledHeight() - 36) return false;
      for (int j = i + 1; j < rectangles.size(); j++) {
        var s = rectangles.get(j);
        if (r[0] < s[0] + s[2] && r[0] + r[2] > s[0] && r[1] < s[1] + s[3] && r[1] + r[3] > s[1])
          return false;
      }
    }
    return true;
  }

  private static net.minecraft.server.level.ServerPlayer serverPlayer() {
    return Minecraft.getInstance().getSingleplayerServer().getPlayerList().getPlayers().getFirst();
  }

  private static void addActionChecks() {
    var mc = Minecraft.getInstance();
    add(
        "Ordinary inventory fixture arrives without modifying personal saves",
        () -> {
          command("clear @a");
          command("item replace entity @a hotbar.0 with golden_pickaxe");
          command("setblock 0 99 0 diamond_ore");
          command("item replace entity @a hotbar.1 with diamond_pickaxe");
          mc.player.getInventory().setSelectedSlot(0);
          mc.player.setXRot(90);
        },
        () ->
            mc.player.getInventory().getItem(1).is(net.minecraft.world.item.Items.DIAMOND_PICKAXE),
        10);
    add(
        "Auto Tool replaces a fast ineligible golden pickaxe with a harvest-correct tool",
        () -> {
          RustyClient.set("auto_tool", true);
          mc.options.keyAttack.setDown(true);
          mc.hitResult =
              new net.minecraft.world.phys.BlockHitResult(
                  new Vec3(.5, 100, .5),
                  net.minecraft.core.Direction.UP,
                  new BlockPos(0, 99, 0),
                  false);
        },
        () -> mc.player.getInventory().getSelectedSlot() == 1,
        2);
    add(
        "Manual hotbar choice retains ownership until mining is released",
        () -> mc.player.getInventory().setSelectedSlot(3),
        () -> mc.player.getInventory().getSelectedSlot() == 3,
        4);
    add(
        "Automatic eating consumes ordinary food on the integrated server",
        () -> {
          mc.options.keyAttack.setDown(false);
          RustyClient.set("auto_tool", false);
          command("item replace entity @a hotbar.4 with apple 2");
          mc.getSingleplayerServer().execute(() -> serverPlayer().getFoodData().setFoodLevel(10));
          RustyClient.set("auto_eat", true);
        },
        () ->
            serverPlayer().getFoodData().getFoodLevel() > 10
                && serverPlayer().getInventory().getItem(4).getCount() < 2,
        40);
    add(
        "Eating restores the previous slot after consuming one item",
        () -> RustyClient.set("auto_eat", false),
        () -> !RustyAssistance.eating() && mc.player.getInventory().getSelectedSlot() == 3,
        5);
    add(
        "Auto Totem fills only an empty offhand through the server",
        () -> {
          command("item replace entity @a weapon.offhand with air");
          command("item replace entity @a hotbar.5 with totem_of_undying");
          RustyClient.set("auto_totem", true);
        },
        () -> serverPlayer().getOffhandItem().is(net.minecraft.world.item.Items.TOTEM_OF_UNDYING),
        15);
    add(
        "Auto Totem preserves an occupied shield offhand",
        () -> {
          command("item replace entity @a weapon.offhand with shield");
          command("item replace entity @a hotbar.5 with totem_of_undying");
        },
        () ->
            serverPlayer().getOffhandItem().is(net.minecraft.world.item.Items.SHIELD)
                && serverPlayer()
                    .getInventory()
                    .getItem(5)
                    .is(net.minecraft.world.item.Items.TOTEM_OF_UNDYING),
        25);
    add(
        "Armor Assist fills an empty armor slot using an ordinary inventory action",
        () -> {
          RustyClient.set("auto_totem", false);
          command("item replace entity @a armor.head with air");
          command("item replace entity @a inventory.1 with iron_helmet");
          RustyClient.set("armor_assist", true);
        },
        () ->
            serverPlayer()
                .getItemBySlot(net.minecraft.world.entity.EquipmentSlot.HEAD)
                .is(net.minecraft.world.item.Items.IRON_HELMET),
        15);
    add(
        "Refill observes a real baseline stack",
        () -> {
          RustyClient.set("armor_assist", false);
          command("item replace entity @a hotbar.8 with stone 1");
          command("item replace entity @a inventory.2 with stone 8");
          RustyClient.set("refill", true);
        },
        () -> mc.player.getInventory().getItem(8).is(net.minecraft.world.item.Items.STONE),
        15);
    add(
        "Refill moves an exact matching stack after observed depletion",
        () -> command("item replace entity @a hotbar.8 with air"),
        () ->
            serverPlayer().getInventory().getItem(8).is(net.minecraft.world.item.Items.STONE)
                && serverPlayer().getInventory().getItem(8).getCount() == 8,
        15);
    add(
        "Identical authoritative correction is not blindly retried",
        () -> {
          command("item replace entity @a hotbar.8 with air");
          command("item replace entity @a inventory.2 with stone 8");
        },
        () ->
            serverPlayer().getInventory().getItem(8).isEmpty()
                && serverPlayer().getInventory().getItem(11).getCount() == 8,
        25);
    add(
        "Explicit refill rearm retains its observed depletion candidate",
        () -> {
          RustyClient.set("refill", false);
          RustyClient.set("refill", true);
        },
        () ->
            serverPlayer().getInventory().getItem(8).is(net.minecraft.world.item.Items.STONE)
                && serverPlayer().getInventory().getItem(8).getCount() == 8,
        15);
    add(
        "Trigger fixture is an owned non-player living entity",
        () -> {
          RustyClient.set("refill", false);
          command("kill @e[type=minecraft:pig]");
          command("summon pig 1 100 1 {NoAI:1b,Tags:[\"rusty_qa_target\"]}");
          mc.player.setXRot(0);
        },
        () -> {
          for (var e : mc.level.entitiesForRendering())
            if (e instanceof net.minecraft.world.entity.animal.pig.Pig pig
                && pig.isAlive()
                && pig.distanceToSqr(mc.player) < 9) {
              combatTarget = pig;
              return true;
            }
          return false;
        },
        10);
    add(
        "Trigger uses an ordinary full-cooldown reachable attack",
        () -> {
          combatHealth = combatTarget.getHealth();
          Vec3 delta =
              combatTarget.getBoundingBox().getCenter().subtract(mc.player.getEyePosition());
          mc.player.setYRot((float) Math.toDegrees(Math.atan2(-delta.x, delta.z)));
          mc.player.setXRot(
              (float) -Math.toDegrees(Math.atan2(delta.y, Math.hypot(delta.x, delta.z))));
          mc.hitResult = new net.minecraft.world.phys.EntityHitResult(combatTarget);
          RustyClient.set("trigger_bot", true);
        },
        () ->
            mc.getSingleplayerServer()
                        .getAllLevels()
                        .iterator()
                        .next()
                        .getEntity(combatTarget.getUUID())
                    instanceof net.minecraft.world.entity.LivingEntity target
                && target.getHealth() < combatHealth,
        1);
    add(
        "Automatic actions stop when the Minecraft menu opens",
        () -> {
          RustyClient.set("trigger_bot", false);
          mc.gui.setScreen(new RustyScreen());
        },
        () -> !RustyAssistance.eating(),
        5);
    add(
        "Action fixtures leave normal controls usable",
        () -> {
          mc.gui.setScreen(null);
          mc.player.setXRot(25);
          mc.player.setYRot(0);
        },
        () -> mc.gui.screen() == null,
        5);
  }

  private static void prepareRemote() {
    var mc = Minecraft.getInstance();
    addCodecChecks();
    if (MODE.equals("remote-denied")) {
      add(
          "Vanilla server joins without requiring Rusty",
          () -> {},
          () -> !mc.hasSingleplayerServer() && mc.player != null,
          40);
      add(
          "No server permission fails closed for restricted tools",
          () -> {
            for (String id :
                List.of(
                    "xray",
                    "finder",
                    "entity_esp",
                    "storage_esp",
                    "freecam",
                    "trigger_bot",
                    "safe_walk")) RustyClient.set(id, true);
          },
          () ->
              !RustyClient.active("xray")
                  && !RustyClient.active("freecam")
                  && !RustyClient.active("trigger_bot")
                  && !RustyClient.active("safe_walk"),
          20);
      add(
          "Ordinary client information remains available on vanilla server",
          () -> {
            RustyClient.set("coordinates", true);
            RustyClient.set("fps_hud", true);
          },
          () ->
              RustyHud.bounds.containsKey("coordinates") && RustyHud.bounds.containsKey("fps_hud"),
          20);
    } else {
      add(
          "Server opt-in payload grants only its declared capabilities",
          () -> {},
          () -> RustyClient.POLICY.allows("xray") && !RustyClient.POLICY.allows("trigger_bot"),
          80);
      add(
          "Received permission enables real target scanning",
          () -> {
            RustyClient.config.scanRadius = 16;
            RustyClient.config.minY = 96;
            RustyClient.config.maxY = 104;
            RustyClient.set("xray", true);
          },
          () -> !RustyClient.targets.isEmpty() && RustyClient.view.xray(),
          30);
      add(
          "Server revocation stops restricted rendering immediately",
          () -> {
            try {
              Files.writeString(
                  mc.gameDirectory.toPath().resolve("rusty-qa/revoke-requested"), "ready");
            } catch (Exception e) {
              throw new IllegalStateException(e);
            }
          },
          () -> !RustyClient.POLICY.allows("xray") && !RustyClient.view.xray(),
          20);
    }
  }

  private static void addCodecChecks() {
    add(
        "Permission payload rejects invalid versions, masks and lengths",
        () -> {
          for (byte[] data : new byte[][] {{2, 1}, {1, 16}, {1}, {1, 1, 0}}) {
            var buf =
                new net.minecraft.network.FriendlyByteBuf(
                    io.netty.buffer.Unpooled.wrappedBuffer(data));
            boolean rejected = false;
            try {
              dev.justeck.rusty.network.ConsentPayload.CODEC.decode(buf);
            } catch (IllegalArgumentException e) {
              rejected = true;
            } finally {
              buf.release();
            }
            if (!rejected) throw new AssertionError("Malformed permission accepted");
          }
          var buf =
              new net.minecraft.network.FriendlyByteBuf(
                  io.netty.buffer.Unpooled.wrappedBuffer(new byte[] {1, 1}));
          try {
            if (dev.justeck.rusty.network.ConsentPayload.CODEC.decode(buf).mask() != 1)
              throw new AssertionError("Valid view grant failed");
          } finally {
            buf.release();
          }
        },
        () -> true,
        1);
  }

  private static void write(boolean finalResult, String error) {
    try {
      var mc = Minecraft.getInstance();
      Path directory = mc.gameDirectory.toPath().resolve("rusty-qa");
      Files.createDirectories(directory);
      var receipt = new LinkedHashMap<String, Object>();
      receipt.put("mode", MODE);
      receipt.put("edition", System.getProperty("rusty.edition"));
      receipt.put("finished", finalResult);
      receipt.put("passed", finalResult && error == null);
      receipt.put("error", error);
      receipt.put("time", Instant.now().toString());
      receipt.put("checks", checks);
      receipt.put("stage", stage);
      receipt.put("ticks", ticks);
      if (mc.player != null) {
        receipt.put("body_position", List.of(mc.player.getX(), mc.player.getY(), mc.player.getZ()));
        receipt.put(
            "detached_controller",
            List.of(RustyCamera.position.x, RustyCamera.position.y, RustyCamera.position.z));
        var camera = mc.gameRenderer.mainCamera().position();
        receipt.put("rendered_camera", List.of(camera.x, camera.y, camera.z));
      }
      receipt.put("framed_quads_emitted", TextureFrames.framedQuads.get());
      receipt.put("rendered_overlay_geometry", RustyOverlay.renderedGeometry);
      receipt.put("performance_samples", performance);
      Files.writeString(directory.resolve("acceptance.json"), RustyClient.GSON.toJson(receipt));
    } catch (Exception e) {
      RustyClient.LOG.error("Could not write game receipt", e);
    }
  }

  private static void finish(boolean passed, String error) {
    finished = true;
    write(true, passed ? null : error);
    RustyClient.LOG.info("Rusty QA completed: {} checks, passed={}", checks.size(), passed);
    var mc = Minecraft.getInstance();
    mc.options.keyUp.setDown(false);
    mc.options.keyAttack.setDown(false);
    mc.options.keyUse.setDown(false);
    if (!Boolean.getBoolean("rusty.qa.keepOpen")) mc.stop();
  }
}
