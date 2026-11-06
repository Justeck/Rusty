package dev.justeck.rusty.mixin;

import dev.justeck.rusty.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.player.ClientInput;
import net.minecraft.client.player.KeyboardInput;
import net.minecraft.world.entity.player.Input;
import net.minecraft.world.phys.Vec2;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(KeyboardInput.class)
public abstract class InputMixin extends ClientInput {
  @Inject(method = "tick", at = @At("TAIL"))
  private void rusty$normalInput(CallbackInfo ci) {
    var mc = Minecraft.getInstance();
    var p = mc.player;
    if (p == null || mc.level == null) return;
    if (mc.gui.screen() != null || !mc.isWindowActive() || RustyClient.active("freecam")) {
      keyPresses = Input.EMPTY;
      moveVector = Vec2.ZERO;
      RustyNavigation.cancelWalk();
      RustyMining.stop("Menu opened or focus changed");
      return;
    }
    boolean forward = keyPresses.forward(),
        jump = keyPresses.jump(),
        shift = keyPresses.shift(),
        sprint = keyPresses.sprint();
    if (RustyMining.active()
        && (forward
            || keyPresses.backward()
            || keyPresses.left()
            || keyPresses.right()
            || jump
            || shift
            || sprint)) RustyMining.manual();
    if (RustyMining.active()) RustyMining.guard();
    if (RustyMining.walking()) {
      forward = true;
      jump = RustyMining.steppingUp();
      sprint = false;
      moveVector = new Vec2(0, 1);
    }
    if (RustyNavigation.walking()) {
      if (forward
          || keyPresses.backward()
          || keyPresses.left()
          || keyPresses.right()
          || jump
          || shift) RustyNavigation.cancelWalk();
      else {
        forward = true;
        moveVector = new Vec2(0, 1);
      }
    }
    if (!RustyMining.active() && RustyClient.active("auto_sprint") && forward && !shift)
      sprint = true;
    if (p.onGround()
        && !jump
        && !shift
        && (forward || keyPresses.backward() || keyPresses.left() || keyPresses.right())) {
      double angle = Math.toRadians(p.getYRot());
      double f = moveVector.y, s = moveVector.x;
      Vec3 step =
          new Vec3(
                  -Math.sin(angle) * f + Math.cos(angle) * s,
                  0,
                  Math.cos(angle) * f + Math.sin(angle) * s)
              .scale(.45);
      if (!RustyMining.active()
          && RustyClient.active("safe_walk")
          && mc.level.noCollision(p, p.getBoundingBox().move(step).move(0, -.6, 0))) shift = true;
      if (!RustyMining.active()
          && (RustyClient.active("auto_jump") || RustyNavigation.walking())
          && !mc.level.noCollision(p, p.getBoundingBox().move(step))
          && mc.level.noCollision(p, p.getBoundingBox().move(step).move(0, 1.1, 0))) jump = true;
    }
    keyPresses =
        new Input(
            forward,
            keyPresses.backward(),
            keyPresses.left(),
            keyPresses.right(),
            jump,
            shift,
            sprint);
  }
}
