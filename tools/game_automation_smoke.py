#!/usr/bin/env python3
"""Verify ordinary fishing and empty-offhand totem actions in a fresh world.

The fishing case waits for a natural server bite. It never writes bobber state
or creates synthetic nibble metadata. Loot and hand contents are received client
state; an independent Java-server fixture supplies separate server readback.
"""
from __future__ import annotations
import argparse
from pathlib import Path
import subprocess
import sys
import time
from game_assistance_smoke import AssistanceSmoke
from game_gui_smoke import Smoke, AcceptanceFailure, digest


class AutomationSmoke(AssistanceSmoke):
    def __init__(self,args):
        super().__init__(args)
        self.report.update(suite="native-item-automation",script_sha256=digest(Path(__file__)),requested_cases=args.cases,
                           claims="Natural received bite metadata, ordinary actions, received loot/hand state and user cancellation in a controlled integrated world.",
                           not_exercised=["Separate Java-server authority is verified by the dedicated multiplayer fixture, not this integrated test.","High latency, rejected swaps and other players' bobbers are not exercised by this fixture."])

    def automation(self):
        result=self.command("rusty-status").get("automations")
        if not isinstance(result,dict):
            raise AcceptanceFailure("Native automation status interface is absent")
        return result

    def off(self):
        super().off()
        for key in ["auto_fish","auto_totem"]:
            if self.config()["assistance"].get(key,False):
                self.set_module(key,["assistance",key],False)

    def setting(self,key,enabled):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda state:state["open"] and bool(state["hits"]),"automation settings")
        self.search("auto fish")
        self.click('Select("auto_fish")')
        if self.config()["assistance"][key]!=enabled:
            self.click(f'Bool("{key}")',pane="detail")
        self.wait_for(self.config,lambda cfg:cfg["assistance"][key]==enabled,"saved automation setting")
        self.key("key.keyboard.escape")
        self.wait_for(self.ui,lambda state:not state["open"],"closed automation settings")

    def totem(self):
        self.setup()
        for text in ["item replace entity @s hotbar.0 with minecraft:stick",
                     "item replace entity @s hotbar.4 with minecraft:totem_of_undying",
                     "item replace entity @s weapon.offhand with minecraft:shield"]:
            self.server(text)
        self.command("hotbar 1")
        original=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["offhand"].get("item")=="minecraft:shield" and inv["hotbar"][4].get("item")=="minecraft:totem_of_undying","received protected offhand and hotbar totem")
        self.set_module("auto_totem",["assistance","auto_totem"])
        time.sleep(.6)
        protected=self.command("inventory")
        self.require(protected["offhand"].get("item")=="minecraft:shield" and protected["selected"]==1,"AutoTotem replaced a protected nonempty offhand or changed selection")
        self.server("item replace entity @s weapon.offhand with minecraft:air")
        equipped=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["offhand"].get("item")=="minecraft:totem_of_undying" and inv["selected"]==1,"authoritative native hand swap and owned selection restore",timeout=8)
        self.require(equipped["hotbar"][4].get("item") in (None,"minecraft:air"),"Source hotbar slot did not receive the swap's empty stack")
        self.require(equipped["counts"].get("minecraft:totem_of_undying")==1,"Totem count was lost or duplicated")
        self.passed("AutoTotem protects other offhand items and uses a normal confirmed empty-offhand swap",initial=original,protected=protected,equipped=equipped,automation=self.automation())

    def fishing(self):
        self.setup()
        for text in ["fill -4 196 2 4 196 12 stone","fill -4 197 2 4 199 12 water",
                     'item replace entity @s hotbar.0 with minecraft:fishing_rod[minecraft:enchantments={"minecraft:lure":3}]',
                     "item replace entity @s hotbar.1 with minecraft:stick"]:
            self.server(text)
        for position in [(0,199,5),(-3,198,4),(3,198,9)]:
            self.wait_for(lambda p=position:self.command("block "+" ".join(map(str,p))),lambda b:b["loaded"] and b["block"]=="minecraft:water","received natural fishing pond")
        self.command("hotbar 1")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv["held"].get("item")=="minecraft:fishing_rod","received main-hand fishing rod")
        self.setting("fish_recast",True)
        self.set_module("auto_fish",["assistance","auto_fish"])
        time.sleep(.4)
        self.require(self.automation()["own_hook"] is None,"AutoFish cast before an observed user cast")
        self.command("look 0 25")
        self.command("mouse right click")
        cast=self.wait_for(self.automation,lambda state:state["own_hook"] is not None,"received manually cast own bobber",timeout=8)
        own_id=cast["own_hook"]["id"]
        bite=self.wait_for(self.automation,lambda state:state.get("last_bite") is not None and state["last_bite"]["id"]==own_id,"natural server-provided biting metadata",timeout=120)
        recast=self.wait_for(self.automation,lambda state:state["own_hook"] is not None and state["own_hook"]["id"]!=own_id,"confirmed reel removal followed by delayed new bobber",timeout=15)
        inventory=self.wait_for(lambda:self.command("inventory"),lambda inv:any(item!="minecraft:fishing_rod" and item!="minecraft:stick" and count>0 for item,count in inv["counts"].items()),"received ordinary fishing loot",timeout=15)
        self.command("hotbar 2")
        cancelled=self.wait_for(self.automation,lambda state:state["fish_phase"]=="waiting_for_user_cast","manual slot selection cancels automatic fishing",timeout=3)
        time.sleep(.4)
        self.require(self.command("inventory")["selected"]==2,"AutoFish overrode the user's selected slot")
        self.passed("AutoFish waits for real user cast and natural bite, reels once, receives loot and recasts after removal",manual_cast=cast,natural_bite=bite,recast=recast,inventory=inventory,cancelled=cancelled)

    def exercise(self):
        self.launch()
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda state:state["open"] and bool(state["hits"]),"initial automation browser")
        self.search("auto totem")
        self.click('Select("auto_totem")')
        self.wait_for(self.config,lambda cfg:"auto_totem" in cfg["assistance"],"persisted automation profile")
        self.key("key.keyboard.escape")
        for text in ["gamerule advance_time false","gamerule advance_weather false","gamerule spawn_mobs false","difficulty hard","time set noon","weather clear"]:
            self.server(text)
        for key,case in [("auto_totem",self.totem),("auto_fish",self.fishing)]:
            if key not in self.args.cases:continue
            self.phase=key
            self.off();case();self.command("release")
        self.off()

    def finish(self):
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"automation-acceptance.json")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary",required=True,type=Path)
    parser.add_argument("--resources",required=True,type=Path)
    parser.add_argument("--output",required=True,type=Path)
    parser.add_argument("--width",default=960,type=int)
    parser.add_argument("--height",default=540,type=int)
    parser.add_argument("--startup-timeout",default=300,type=float)
    parser.add_argument("--cases",nargs="+",choices=["auto_totem","auto_fish"],default=["auto_totem","auto_fish"])
    args=parser.parse_args();smoke=None
    try:
        smoke=AutomationSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:
            smoke.report["passed"]=False
            smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()

if __name__=="__main__":raise SystemExit(main())
