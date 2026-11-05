package dev.justeck.rusty;

import dev.justeck.rusty.mixin.FishingAccessor;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.ContainerInput;
import net.minecraft.world.item.*;
import net.minecraft.world.item.enchantment.Enchantments;
import net.minecraft.world.phys.*;

/** Ordinary game action paths; menus, manual hotbar changes and focus cancel ownership. */
public final class RustyAssistance {
  private static int clock,
      cooldown,
      eatSlot = -1,
      previousSlot = -1,
      fishCountdown,
      eatStarted,
      toolSlot = -1;
  private static net.minecraft.client.player.LocalPlayer eatOwner;
  private static boolean mining, manualTool;
  private static UUID hook;
  private static boolean reeled;
  private static ItemStack eating = ItemStack.EMPTY;
  private static final ItemStack[] baseline = new ItemStack[9];
  private static final ItemStack[] refillPending = new ItemStack[9];
  private static final LinkedHashSet<String> consumed = new LinkedHashSet<>();

  /** Re-enabling a module explicitly renews its intent, including a rejected/cancelled action. */
  public static void rearm(String id) {
    String prefix =
        switch (id) {
          case "auto_eat" -> "eat:";
          case "auto_totem" -> "totem:";
          case "armor_assist" -> "armor:";
          case "refill" -> "refill";
          default -> "";
        };
    if (!prefix.isEmpty()) consumed.removeIf(k -> k.startsWith(prefix));
  }

  public static boolean eating() {
    return eatSlot >= 0;
  }

  public static void reset() {
    cancelEat();
    Arrays.fill(baseline, null);
    Arrays.fill(refillPending, null);
    consumed.clear();
    hook = null;
    reeled = false;
    cooldown = 0;
    mining = false;
    manualTool = false;
    toolSlot = -1;
  }

  private static String intent(String type, int slot, ItemStack stack) {
    return type
        + ":"
        + slot
        + ":"
        + BuiltInRegistries.ITEM.getKey(stack.getItem())
        + ":"
        + stack.getComponentsPatch().hashCode()
        + ":"
        + stack.getCount();
  }

  private static boolean claim(String key) {
    if (!consumed.add(key)) return false;
    while (consumed.size() > 256) consumed.remove(consumed.iterator().next());
    return true;
  }

  private static void cancelEat() {
    var mc = Minecraft.getInstance();
    if (eatSlot >= 0 && mc.player != null && mc.player == eatOwner) {
      if (mc.gameMode != null && mc.player.isUsingItem()) mc.gameMode.releaseUsingItem(mc.player);
      if (mc.player.getInventory().getSelectedSlot() == eatSlot && previousSlot >= 0)
        mc.player.getInventory().setSelectedSlot(previousSlot);
    }
    eatSlot = -1;
    previousSlot = -1;
    eatOwner = null;
    eating = ItemStack.EMPTY;
  }

  public static void tick(boolean focused) {
    clock++;
    if (cooldown > 0) cooldown--;
    var mc = Minecraft.getInstance();
    var p = mc.player;
    if (p == null
        || mc.level == null
        || mc.gameMode == null
        || !focused
        || p.isDeadOrDying()
        || RustyClient.active("freecam")) {
      cancelEat();
      if (p != null && !focused)
        for (int slot = 0; slot < 9; slot++) {
          var stack = p.getInventory().getItem(slot);
          baseline[slot] = stack.isEmpty() ? null : stack.copy();
          if (!stack.isEmpty()) refillPending[slot] = null;
        }
      hook = null;
      mining = false;
      manualTool = false;
      return;
    }
    if (eating()) {
      if (p != eatOwner
          || !RustyClient.active("auto_eat")
          || p.getInventory().getSelectedSlot() != eatSlot
          || p.getFoodData().getFoodLevel() >= 20
          || !ItemStack.isSameItemSameComponents(eating, p.getMainHandItem())
          || p.getMainHandItem().getCount() != eating.getCount()
          || clock - eatStarted >= 100
          || mc.options.keyAttack.isDown()
          || mc.options.keyUse.isDown()) cancelEat();
      return;
    }
    if (p.containerMenu != p.inventoryMenu
        || !p.inventoryMenu.getCarried().isEmpty()
        || p.isUsingItem()) return;
    if (!mc.options.keyAttack.isDown()) {
      mining = false;
      manualTool = false;
      toolSlot = -1;
    } else if (mining && toolSlot != p.getInventory().getSelectedSlot()) manualTool = true;
    if (RustyClient.active("auto_tool")
        && !manualTool
        && mc.options.keyAttack.isDown()
        && mc.hitResult instanceof BlockHitResult hit) {
      var state = mc.level.getBlockState(hit.getBlockPos());
      int best = p.getInventory().getSelectedSlot();
      var current = p.getMainHandItem();
      boolean usable =
          (!state.requiresCorrectToolForDrops() || current.isCorrectToolForDrops(state))
              && (!current.isDamageableItem()
                  || current.getMaxDamage() - current.getDamageValue()
                      > Math.max(1, current.getMaxDamage() / 20));
      float score = usable ? current.getDestroySpeed(state) : -1;
      for (int slot = 0; slot < 9; slot++) {
        var stack = p.getInventory().getItem(slot);
        if (stack.isDamageableItem()
            && stack.getMaxDamage() - stack.getDamageValue()
                <= Math.max(1, stack.getMaxDamage() / 20)) continue;
        float speed = stack.getDestroySpeed(state);
        if (speed > score
            && (!state.requiresCorrectToolForDrops() || stack.isCorrectToolForDrops(state))) {
          score = speed;
          best = slot;
        }
      }
      p.getInventory().setSelectedSlot(best);
      toolSlot = best;
      mining = true;
    }
    if (mc.options.keyAttack.isDown() || mc.options.keyUse.isDown()) return;
    if (RustyClient.active("trigger_bot")
        && p.getAttackStrengthScale(0) >= 1
        && mc.hitResult instanceof EntityHitResult hit
        && hit.getEntity() instanceof LivingEntity target
        && target.isAlive()
        && target != p
        && p.hasLineOfSight(target)
        && p.isWithinEntityInteractionRange(target, 0)
        && !p.isAlliedTo(target)
        && (!(target instanceof Player) || RustyClient.config.triggerPlayers)) {
      mc.gameMode.attack(p, target);
      p.swing(InteractionHand.MAIN_HAND, p.getMainHandItem().getAttackAnimation(), false);
      return;
    }
    if (RustyClient.active("auto_eat")
        && p.getFoodData().getFoodLevel() < RustyClient.config.eatBelow) {
      for (int slot = 0; slot < 9; slot++) {
        var stack = p.getInventory().getItem(slot);
        String id = BuiltInRegistries.ITEM.getKey(stack.getItem()).getPath();
        if (stack.has(DataComponents.FOOD)
            && stack.has(DataComponents.CONSUMABLE)
            && !Set.of(
                    "rotten_flesh",
                    "spider_eye",
                    "poisonous_potato",
                    "pufferfish",
                    "chorus_fruit",
                    "suspicious_stew")
                .contains(id)
            && claim(intent("eat", slot, stack))) {
          previousSlot = p.getInventory().getSelectedSlot();
          eatSlot = slot;
          eatStarted = clock;
          eatOwner = p;
          eating = stack.copy();
          p.getInventory().setSelectedSlot(slot);
          mc.gameMode.useItem(p, InteractionHand.MAIN_HAND);
          return;
        }
      }
    }
    if (RustyClient.active("auto_fish")
        && p.fishing != null
        && p.getMainHandItem().is(Items.FISHING_ROD)) {
      if (!p.fishing.getUUID().equals(hook)) {
        hook = p.fishing.getUUID();
        fishCountdown = 0;
        reeled = false;
      }
      if (!reeled && ((FishingAccessor) p.fishing).rusty$biting()) {
        if (++fishCountdown >= RustyClient.config.fishDelay && cooldown == 0) {
          mc.gameMode.useItem(p, InteractionHand.MAIN_HAND);
          cooldown = 4;
          reeled = true;
          return;
        }
      } else if (!reeled) fishCountdown = 0;
    }
    if (cooldown > 0) return;
    if (RustyClient.active("auto_totem") && p.getOffhandItem().isEmpty()) {
      for (int slot = 0; slot < 9; slot++) {
        var stack = p.getInventory().getItem(slot);
        if (stack.is(Items.TOTEM_OF_UNDYING) && claim(intent("totem", slot, stack))) {
          mc.gameMode.handleContainerInput(
              p.inventoryMenu.containerId, 36 + slot, 40, ContainerInput.SWAP, p);
          cooldown = 10;
          return;
        }
      }
    }
    if (RustyClient.active("armor_assist")) {
      for (int slot = 9; slot < 36; slot++) {
        var stack = p.getInventory().getItem(slot);
        var equip = stack.get(DataComponents.EQUIPPABLE);
        if (equip == null || !equip.slot().isArmor() || !p.getItemBySlot(equip.slot()).isEmpty())
          continue;
        var ench = stack.get(DataComponents.ENCHANTMENTS);
        if (ench != null
            && ench.entrySet().stream().anyMatch(e -> e.getKey().is(Enchantments.BINDING_CURSE)))
          continue;
        if (claim(intent("armor", slot, stack))) {
          mc.gameMode.handleContainerInput(
              p.inventoryMenu.containerId, slot, 0, ContainerInput.QUICK_MOVE, p);
          cooldown = 10;
          return;
        }
      }
    }
    for (int hotbar = 0; hotbar < 9; hotbar++) {
      var current = p.getInventory().getItem(hotbar);
      if (!current.isEmpty()) {
        baseline[hotbar] = current.copy();
        refillPending[hotbar] = null;
        continue;
      }
      var old = baseline[hotbar];
      baseline[hotbar] = null;
      if (old != null && RustyClient.active("refill")) refillPending[hotbar] = old;
      old = refillPending[hotbar];
      if (old == null) continue;
      if (!RustyClient.active("refill")) continue;
      for (int slot = 9; slot < 36; slot++) {
        var source = p.getInventory().getItem(slot);
        if (ItemStack.isSameItemSameComponents(source, old)
            && claim(intent("refill" + hotbar, slot, source))) {
          mc.gameMode.handleContainerInput(
              p.inventoryMenu.containerId, slot, hotbar, ContainerInput.SWAP, p);
          cooldown = 10;
          return;
        }
      }
    }
  }
}
