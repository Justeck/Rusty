#!/usr/bin/env python3
"""Auto Mine acceptance in owned survival terrain; never opens a personal world/server."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time
from game_feature_smoke import FeatureSmoke
from game_gui_smoke import AcceptanceFailure, Smoke, digest


class MiningSmoke(FeatureSmoke):
    native_terrain = True

    def __init__(self, args):
        super().__init__(args)
        self.report.update(suite="native-auto-mine", script_sha256=digest(Path(__file__)),
            claims="Actual owned survival movement and normal block breaking; resource counts require observed empty blocks and inventory readbacks.",
            not_exercised=["Personal servers/worlds, server anticheats, server-hidden ores and arbitrary latency are not covered.","Physical hardware and long-session soak testing are outside this fixture."])
        for key in ["pixel_contract", "fixture_correction"]:
            self.report.pop(key, None)
        self.config_path.parent.mkdir(parents=True, exist_ok=True)
        self.config_path.write_text(json.dumps({"mining":{"radius":8,"goal":1}}))

    def miner(self):
        return self.command("rusty-mine-status")

    def player(self):
        return self.command("state")["player"]

    def fixture(self, tunnel=False, hazard=None):
        if self.miner()["active"]:
            self.command("rusty-mine-toggle")
        self.command("release")
        for cmd in ["gamemode spectator @s","tp @s 0.5 210 0.5 0 0","fill -3 200 -3 3 205 12 air","fill -3 199 -3 3 199 12 bedrock",
                    "fill -1 200 -1 -1 202 10 bedrock","fill 1 200 -1 1 202 10 bedrock","clear @s","kill @e[type=minecraft:item]",
                    "item replace entity @s hotbar.0 with minecraft:diamond_pickaxe","setblock 0 200 7 minecraft:diamond_ore"]:
            self.server(cmd)
        if tunnel:
            self.server("setblock 0 200 3 minecraft:stone")
            self.server("setblock 0 201 3 minecraft:stone")
        if hazard:
            if hazard=="minecraft:water":
                self.server("setblock 0 200 9 bedrock")
                self.server("setblock 0 201 8 bedrock")
                self.server("setblock 0 200 8 minecraft:water")
            else:
                self.server("setblock 0 202 7 bedrock")
                self.server(f"setblock 0 201 7 {hazard}")
        self.server("effect give @s minecraft:instant_health 1 10")
        self.server("effect give @s minecraft:saturation 1 10")
        self.server("tp @s 0.5 200 0.5 0 0")
        self.server("gamemode survival @s")
        self.wait_for(self.player,lambda p:p["on_ground"] and not p["flying"] and abs(p["position"][1]-200)<.05,"grounded survival miner",timeout=30)
        self.wait_for(lambda:self.command("block 0 200 7"),lambda b:"diamond_ore" in str(b),"received target ore",timeout=15)
        time.sleep(.3)

    def begin(self, ui=False):
        if ui:
            self.key("key.keyboard.right.shift")
            self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"Auto Mine browser")
            self.search("auto mine")
            self.click('Select("auto_mine")')
            self.shot("auto-mine-settings")
            self.click('Toggle("auto_mine")')
        else:
            self.command("rusty-mine-toggle")
        self.wait_for(self.miner,lambda s:s["active"],"explicit mining session",timeout=5)

    def completed(self):
        state=self.wait_for(self.miner,lambda s:not s["active"],"mining session ends",timeout=45)
        self.require(state["count"]==1 and state["result"]=="Block goal reached","Mining did not meet its block goal: "+str(state))
        self.require("air" in str(self.command("block 0 200 7")),"Resource block was not actually removed")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv.get("counts",{}).get("minecraft:diamond",0)>=1,"normal collected ore drop",timeout=5)
        self.require(self.player()["position"][2]>=7,"The supported collection step was not completed")
        self.require(self.command("state")["busy"] is None,"Completed mining retained control ownership")
        return state

    def exercise(self):
        self.launch()
        for cmd in ["gamerule advance_time false","gamerule spawn_mobs false","gamerule keep_inventory true","time set noon","weather clear"]:
            self.server(cmd)
        self.load_area(-3,-3,3,12,y=210)
        self.phase="normal survival ore collection"
        self.fixture();self.begin(ui=True)
        state=self.completed();self.passed("Menu starts Auto Mine; survival walking, normal ore break, drop pickup and quota stop",result=state,player=self.player())
        self.shot("auto-mine-collected")

        self.phase="safe access tunnel"
        self.fixture(tunnel=True);self.begin();state=self.completed()
        self.require("air" in str(self.command("block 0 200 3")) and "air" in str(self.command("block 0 201 3")),"Access tunnel did not clear both head and feet")
        self.passed("Conservative access tunnel clears safe ordinary terrain before mining ore",result=state)

        self.phase="harvest-correct tool and selection restoration"
        self.fixture()
        self.server("item replace entity @s hotbar.0 with minecraft:golden_pickaxe")
        self.server("item replace entity @s hotbar.1 with minecraft:diamond_pickaxe")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv.get("counts",{}).get("minecraft:golden_pickaxe",0)==1 and inv.get("counts",{}).get("minecraft:diamond_pickaxe",0)==1,"two eligible-speed but different-harvest-tier tools")
        self.begin();state=self.completed()
        self.require((self.command("inventory").get("held") or {}).get("item")=="minecraft:golden_pickaxe","Auto Mine did not restore the previous selected slot")
        self.passed("A harvest-correct tool replaces an ineligible golden pickaxe and the previous selection is restored",result=state)

        self.phase="worn tool refusal"
        self.fixture();self.server("item replace entity @s hotbar.0 with minecraft:diamond_pickaxe[minecraft:damage=1550]")
        self.wait_for(lambda:self.command("inventory"),lambda inv:'"damage": 1550' in json.dumps(inv),"received nearly broken pickaxe")
        self.begin();state=self.wait_for(self.miner,lambda s:not s["active"],"worn tool prevents mining",timeout=30)
        self.require(state["count"]==0 and "diamond_ore" in str(self.command("block 0 200 7")),"Worn tool was used to mine a resource")
        self.passed("Tools below the durability reserve do not start resource actions",result=state)

        self.phase="full inventory refusal"
        self.fixture()
        for slot in range(36):
            target="hotbar."+str(slot) if slot<9 else "inventory."+str(slot-9)
            self.server("item replace entity @s "+target+" with minecraft:wooden_pickaxe")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv.get("counts",{}).get("minecraft:wooden_pickaxe",0)==36,"received full survival inventory")
        rejected=False
        try:self.command("rusty-mine-toggle")
        except AcceptanceFailure:rejected=True
        self.require(rejected and not self.miner()["active"],"Auto Mine accepted a full inventory")
        self.passed("Full inventory rejects an explicit start before any movement")

        self.phase="reprogrammable explicit shortcut"
        self.fixture();self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"shortcut settings")
        self.search("auto mine");self.click('Select("auto_mine")');self.click('Bind("auto_mine")')
        self.key("key.keyboard.k")
        self.wait_for(self.ui,lambda s:s.get("capture") is None,"shortcut capture finishes")
        self.key("key.keyboard.escape");self.key("key.keyboard.k")
        self.wait_for(self.miner,lambda s:s["active"],"rebound shortcut starts mining")
        self.key("key.keyboard.k")
        self.wait_for(self.miner,lambda s:not s["active"],"same shortcut stops mining")
        time.sleep(.5);self.require(not self.miner()["active"],"Explicit shortcut stop restarted itself")
        self.passed("The menu rebinds Auto Mine; its shortcut explicitly starts and stops without resuming")

        for hazard in ["minecraft:water","minecraft:gravel"]:
            self.phase="refuse hazardous mining face: "+hazard
            self.fixture(hazard=hazard);before=self.player()["position"];self.begin()
            state=self.wait_for(self.miner,lambda s:not s["active"],"unsafe resource refused",timeout=30)
            self.require(state["count"]==0 and "diamond_ore" in str(self.command("block 0 200 7")),"Hazardous resource was mined")
            after=self.player()["position"];self.require(sum((after[i]-before[i])**2 for i in range(3))<.01,"Unusable target triggered movement")
            self.passed("Hazard blocks route execution before any movement or mining",hazard=hazard,result=state)

        self.phase="manual takeover"
        self.fixture();self.begin();self.key("key.keyboard.w","down")
        try:
            state=self.wait_for(self.miner,lambda s:not s["active"],"manual movement cancels Auto Mine")
            before=self.player()["position"];time.sleep(.4);after=self.player()["position"]
            self.require(after[2]>before[2]+.1,"Auto Mine swallowed manual forward input")
        finally:
            self.key("key.keyboard.w","up");self.command("release")
        self.passed("Manual movement cancels automation and retains the user's held key",result=state)

        self.phase="menu cancellation"
        self.fixture();self.begin();self.key("key.keyboard.right.shift")
        self.wait_for(self.miner,lambda s:not s["active"],"menu cancels mining")
        self.key("key.keyboard.escape");time.sleep(.5)
        self.require(not self.miner()["active"],"Closing menu restarted mining")
        self.passed("Menus cancel mining and closing them never resumes it")

        self.phase="damage cancellation"
        self.fixture();self.begin();self.server("damage @s 1 minecraft:generic")
        state=self.wait_for(self.miner,lambda s:not s["active"],"damage stops mining")
        self.require(state["count"]==0,"Damage stop reported unexpected mining")
        self.passed("Observed damage immediately stops the mode",result=state)

        self.phase="server denied mining"
        self.fixture();self.server("gamemode adventure @s");self.begin()
        state=self.wait_for(self.miner,lambda s:not s["active"],"denied break terminates",timeout=45)
        self.require(state["count"]==0 and "diamond_ore" in str(self.command("block 0 200 7")),"Denied break falsely reported success")
        self.passed("Server-denied break times out without counting a resource or retrying forever",result=state)

        self.phase="dimension transition cancellation"
        self.fixture();self.begin()
        self.server("execute in minecraft:the_nether run tp @s 0.5 200 0.5 0 0")
        state=self.wait_for(self.miner,lambda s:not s["active"],"dimension change cancels mining",timeout=45)
        self.require(state["count"]==0,"Dimension transition retained or completed an old route")
        self.passed("Dimension changes cancel the old session and never resume it",result=state)

    def finish(self):
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"mining-acceptance.json")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ["binary","resources","output"]:parser.add_argument("--"+name,required=True,type=Path)
    parser.add_argument("--width",default=960,type=int);parser.add_argument("--height",default=540,type=int);parser.add_argument("--startup-timeout",default=240,type=float)
    args=parser.parse_args();smoke=None
    try:
        smoke=MiningSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()

if __name__=="__main__":raise SystemExit(main())
