#!/usr/bin/env python3
"""Exercise ordinary assistance actions in a new disposable native world.

The file records behavioral results separately from failures; it never opens an
installed instance or accesses an account/server. All server commands address
the newly created integrated fixture. Screenshots still need visual review.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time
from game_feature_smoke import FeatureSmoke
from game_gui_smoke import AcceptanceFailure, digest


class AssistanceSmoke(FeatureSmoke):
    native_terrain = True

    def __init__(self, args):
        super().__init__(args)
        self.report.update(suite="native-assistance", script_sha256=digest(Path(__file__)),
                           claims="Live ordinary input and received player/inventory/entity assertions; no screenshot art claim.",
                           not_exercised=["External multiplayer, controller remappings, live focus loss, reconnect, high latency and modified item components are not covered.",
                                          "Screenshots require separate visual review. Native collision, slot, food and target-health observations are asserted."],
                           fixture="Fresh offline integrated vanilla world; no test-terrain reload inconsistency.")
        for key in ["pixel_contract","fixture_correction","photo_readiness","paired_camera_receipts","hud_input_transitions"]:
            self.report.pop(key,None)
        self.failed_cases = []
        self.report["requested_cases"]=getattr(args,"cases",["safe_walk","auto_jump","auto_tool","trigger_bot","auto_eat"])

    def player(self):
        return self.command("state")["player"]

    def setup(self):
        self.command("release")
        for cmd in ["effect clear @s", "kill @e[type=minecraft:cow]", "kill @e[type=minecraft:item]", "clear @s",
                    "gamemode spectator @s", "tp @s 0.5 210 0.5 0 0"]:
            self.server(cmd)
        self.wait_for(self.player,lambda p:all(abs(p["position"][i]-[.5,210,.5][i])<.05 for i in range(3)),"spectator fixture load position",timeout=30)
        for x in [-6,6]:
            for z in [-6,12]:
                self.wait_for(lambda x=x,z=z:self.command(f"block {x} 199 {z}"),lambda b:b["loaded"],"received fixture corner chunk",timeout=60)
        self.server("fill -6 200 -6 6 205 12 air")
        self.server("fill -6 198 -6 6 199 12 stone")
        floor=[]
        for x,z in [(-6,-6),(-6,12),(6,-6),(6,12),(0,0),(0,4),(0,5)]:
            observed=self.wait_for(lambda x=x,z=z:self.command(f"block {x} 199 {z}"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","received stone fixture floor",timeout=30)
            floor.append({"position":[x,199,z],"observed":observed})
        self.wait_for(lambda:self.command("block 0 200 0"),lambda b:b["loaded"] and b["block"]=="minecraft:air","received clear fixture body space")
        self.server("tp @s 0.5 200 0.5 0 0")
        self.server("gamemode survival @s")
        grounded=self.wait_for(self.player, lambda p: abs(p["position"][1]-200) < .01 and p["on_ground"] and not p["flying"], "grounded fixture player",timeout=30)
        time.sleep(.25)
        settled=self.player()
        self.require(all(abs(settled["position"][i]-[.5,200,.5][i])<.01 for i in range(3)) and settled["on_ground"],"fixture body was not stable across multiple physics ticks")
        self.report.setdefault("fixture_receipts",[]).append({"case":self.phase,"floor":floor,"grounded":grounded,"settled":settled})

    def off(self):
        self.command("release")
        if self.ui()["open"]:
            self.key("key.keyboard.escape")
        for name in ["safe_walk", "auto_jump", "auto_tool", "trigger_bot", "auto_eat"]:
            if self.config()["assistance"][name]:
                self.set_module(name, ["assistance", name], False)

    def walk(self):
        self.setup()
        self.server("fill -6 198 5 6 199 12 air")
        for z,wanted in [(4,"minecraft:stone"),(5,"minecraft:air"),(12,"minecraft:air")]:
            self.wait_for(lambda z=z:self.command(f"block 0 199 {z}"),lambda b,wanted=wanted:b["loaded"] and b["block"]==wanted,"received ledge support and gap")
        self.set_module("safe_walk", ["assistance", "safe_walk"])
        self.key("key.keyboard.w", "down")
        samples = []
        until = time.monotonic()+3.0
        while time.monotonic() < until:
            samples.append(self.player())
            time.sleep(.08)
        self.command("release")
        self.report.setdefault("observations",{})["safe_walk_samples"]=samples
        self.require(all(p["position"][1] > 199.9 for p in samples), "SafeWalk allowed grounded body to fall from fixture ledge")
        self.require(samples[-1]["position"][2] > 3.0, "SafeWalk did not approach the ledge")
        self.passed("SafeWalk approaches a known ledge and retains body support", final=samples[-1], samples=len(samples))
        self.set_module("safe_walk", ["assistance", "safe_walk"], False)
        self.key("key.keyboard.w", "down")
        dropped = self.wait_for(self.player, lambda p: p["position"][1] < 199.5, "disabled SafeWalk permits ordinary ledge fall", timeout=4)
        self.command("release")
        self.passed("Disabling SafeWalk restores ordinary movement off the same ledge", position=dropped["position"])

    def jump(self):
        self.setup()
        self.server("setblock 0 200 2 stone")
        self.wait_for(lambda:self.command("block 0 200 2"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","received one-block jump obstacle")
        self.key("key.keyboard.w", "down")
        time.sleep(.7)
        baseline = self.player()
        self.command("release")
        self.require(baseline["position"][2] < 1.75 and baseline["position"][1] < 200.1,
                     "AutoJump-off baseline already jumped or traversed the obstacle")
        self.server("tp @s 0.5 200 0.5 0 0")
        self.set_module("auto_jump", ["assistance", "auto_jump"])
        self.key("key.keyboard.w", "down")
        samples=[]
        until=time.monotonic()+1.6
        while time.monotonic()<until:
            samples.append(self.player()); time.sleep(.04)
        self.command("release")
        self.require(any(p["position"][1] > 200.5 for p in samples), "AutoJump did not produce ordinary upward movement at the obstacle")
        self.require(samples[-1]["position"][2]>2.8, "AutoJump did not traverse one-block obstacle")
        self.passed("AutoJump requests a normal jump over a received one-block obstacle", disabled_baseline=baseline, maximum_y=max(p["position"][1] for p in samples), final=samples[-1])

    def tool(self):
        self.setup()
        for cmd in ["fill -1 201 3 1 202 7 stone",
                    "item replace entity @s hotbar.0 with minecraft:wooden_pickaxe",
                    "item replace entity @s hotbar.1 with minecraft:iron_pickaxe",
                    "item replace entity @s hotbar.2 with minecraft:diamond_pickaxe[minecraft:damage=1550]",
                    "item replace entity @s hotbar.3 with minecraft:stick"]:
            self.server(cmd)
        self.command("hotbar 1")
        self.wait_for(lambda:self.command("target"),lambda t:t.get("kind")=="block", "received mining target")
        self.set_module("auto_tool", ["assistance", "auto_tool"])
        self.command("mouse left down")
        chosen=self.wait_for(lambda:self.command("inventory"),lambda i:i["selected"]==2, "AutoTool chooses healthy iron over worn diamond", timeout=3)
        self.command("hotbar 4")
        time.sleep(.35)
        self.require(self.command("inventory")["selected"]==4, "AutoTool overrode manual hotbar selection")
        self.command("mouse left up")
        time.sleep(.15)
        self.require(self.command("inventory")["selected"]==4, "AutoTool restored a stale slot over user's choice")
        self.passed("AutoTool uses host tool rules, durability threshold and manual selection precedence", automatic=chosen["held"], manual_selected=4)

    def trigger(self):
        self.setup()
        self.server('summon minecraft:cow 0.5 200 3 {NoAI:1b,PersistenceRequired:1b}')
        self.command("look 0 15")
        initial=self.wait_for(lambda:self.command("target"),lambda t:t.get("entity") is not None, "received controlled cow crosshair target")
        self.server("fill 0 200 2 0 202 2 stone")
        self.wait_for(lambda:self.command("target"),lambda t:t["kind"]=="block", "blocking wall before mob")
        health=initial["entity"]["health"]
        cow_id=initial["entity"]["id"]
        def cow():
            found = next((e for e in self.command("entities 8 minecraft:cow")["entities"] if e["id"]==cow_id),None)
            if found is None:
                raise AcceptanceFailure("Controlled cow left the received entity set")
            return found
        self.set_module("trigger_bot", ["assistance", "trigger_bot"])
        time.sleep(.65)
        blocked=cow()
        self.require(blocked["health"]==health,"TriggerBot attacked through received wall")
        self.server("fill 0 200 2 0 202 2 air")
        hit=self.wait_for(cow,lambda e:e["health"]<health, "ordinary crosshair attack damages visible controlled mob", timeout=4)
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"],"menu owns input")
        paused=cow()["health"]
        time.sleep(.5)
        self.require(cow()["health"]==paused,"TriggerBot attacked beneath menu")
        self.key("key.keyboard.escape")
        self.passed("TriggerBot uses ordinary visible-target attack and suspends beneath menu", before=health, after=hit["health"], blocked=blocked["health"], menu_paused=paused)

    def eat(self):
        self.setup()
        for cmd in ["item replace entity @s hotbar.0 with minecraft:stick", "item replace entity @s hotbar.1 with minecraft:poisonous_potato 12", "item replace entity @s hotbar.3 with minecraft:bread 12", "setblock 0 201 3 chest", "effect give @s minecraft:hunger 5 255"]:
            self.server(cmd)
        food_samples=[]
        def food_sample():
            value=self.player(); food_samples.append(value)
            self.report.setdefault("observations",{})["hunger_samples"]=food_samples
            return value
        before=self.wait_for(food_sample,lambda p:p["food"]<=16,"ordinary hunger effect makes food eligible",timeout=8)
        self.server("effect clear @s")
        self.command("hotbar 1")
        self.wait_for(lambda:self.command("target"),lambda t:t["kind"]=="block", "crosshair chest before automatic eating")
        self.set_module("auto_eat", ["assistance", "auto_eat"])
        eating_samples=[]
        def eating_sample():
            state=self.command("state")
            inventory=self.command("inventory")
            module=self.command("rusty-status")
            eating_samples.append({"player":state["player"],"using":state.get("using"),"selected":inventory["selected"],"held":inventory["held"],"assistance":module.get("assistance")})
            self.report.setdefault("observations",{})["eating_samples"]=eating_samples
            return state["player"]
        after=self.wait_for(eating_sample,lambda p:p["food"]>before["food"],"server confirms automatic food consumption",timeout=6)
        inv=self.wait_for(lambda:self.command("inventory"),lambda i:i["selected"]==1,"AutoEat restores its original hotbar selection",timeout=3)
        self.require(self.command("state")["screen"]=="playing","AutoEat opened the targeted chest")
        self.require(inv["counts"].get("minecraft:poisonous_potato")==12,"AutoEat consumed excluded poisonous food")
        self.require(inv["counts"].get("minecraft:bread",12)<12,"Food increased without consuming fixture bread")
        self.passed("AutoEat consumes safe hotbar food, ignores poisonous food and never opens targeted chest", food_before=before["food"],food_after=after["food"],inventory=inv)

    def exercise(self):
        self.launch()
        # First module visit persists the default profile. Fresh instances have
        # no config file until the first ordinary preference/action is saved.
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda state:state["open"] and bool(state["hits"]),"initial module browser")
        self.search("safe walk")
        self.click('Select("safe_walk")')
        self.wait_for(self.config,lambda cfg:"assistance" in cfg,"first saved default profile")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui,lambda state:not state["open"],"closed initial module browser")
        for cmd in ["gamerule advance_time false", "gamerule spawn_mobs false", "difficulty hard", "time set noon", "weather clear"]:
            self.server(cmd)
        selected=set(self.report["requested_cases"])
        for key,name,case in [("safe_walk","SafeWalk",self.walk),("auto_jump","AutoJump",self.jump),("auto_tool","AutoTool",self.tool),("trigger_bot","TriggerBot",self.trigger),("auto_eat","AutoEat",self.eat)]:
            if key not in selected:
                continue
            self.phase=name
            try:
                self.off(); case()
            except (AcceptanceFailure,KeyError,ValueError,TypeError) as exc:
                self.failed_cases.append(name)
                self.report["checks"].append({"name":name,"passed":False,"reason":str(exc)})
                print("FAIL "+name+": "+str(exc),flush=True)
            finally:
                self.command("release")
        self.off()
        if self.failed_cases:
            raise AcceptanceFailure("Failed assistance cases: "+", ".join(self.failed_cases))

    def finish(self):
        # Invoke base once: parent FeatureSmoke would rename its evidence.
        from game_gui_smoke import Smoke
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"assistance-acceptance.json")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary",required=True,type=Path)
    parser.add_argument("--resources",required=True,type=Path)
    parser.add_argument("--output",required=True,type=Path)
    parser.add_argument("--width",default=960,type=int)
    parser.add_argument("--height",default=540,type=int)
    parser.add_argument("--startup-timeout",default=240,type=float)
    parser.add_argument("--cases",nargs="+",choices=["safe_walk","auto_jump","auto_tool","trigger_bot","auto_eat"],default=["safe_walk","auto_jump","auto_tool","trigger_bot","auto_eat"])
    args=parser.parse_args()
    smoke=None
    try:
        smoke=AssistanceSmoke(args); smoke.exercise(); smoke.report["passed"]=True; return 0
    except (AcceptanceFailure,OSError,ValueError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:
            smoke.report["passed"]=False
            smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True); return 1
    finally:
        if smoke: smoke.finish()

if __name__=="__main__": raise SystemExit(main())
