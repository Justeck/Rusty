#!/usr/bin/env python3
"""Area Clear acceptance in owned disposable survival terrain only."""
from __future__ import annotations
import argparse, json, math, subprocess, sys, time
from pathlib import Path
from game_mining_smoke import MiningSmoke
from game_gui_smoke import Smoke, AcceptanceFailure, digest

class AreaClearSmoke(MiningSmoke):
    def __init__(self,args):
        super().__init__(args)
        self.report.update(suite="native-area-clear",script_sha256=digest(Path(__file__)),claims="Actual bounded survival excavation with received block readbacks and explicit corner selection.")
    def clear(self): return self.command("rusty-clear-status")
    def fixture(self,material="stone"):
        if self.clear()["active"]: self.command("rusty-clear-toggle")
        self.command("release")
        for cmd in ["gamemode spectator @s","tp @s 0.5 210 0.5 0 0","fill -4 200 -4 5 206 9 air","fill -4 199 -4 5 199 9 bedrock","clear @s","kill @e[type=minecraft:item]","item replace entity @s hotbar.0 with minecraft:diamond_pickaxe",f"fill 0 200 3 1 201 4 {material}","setblock -1 200 3 stone","setblock 2 200 3 stone","effect give @s instant_health 1 10","effect give @s saturation 1 10","tp @s 0.5 200 0.5 0 0","gamemode survival @s"]: self.server(cmd)
        self.wait_for(self.player,lambda p:p["on_ground"] and not p["flying"] and abs(p["position"][1]-200)<.05,"grounded area worker",timeout=30)
        self.wait_for(lambda:self.command("block 1 201 4"),lambda b:material in str(b),"received selection blocks")
        self.corner((0,200,3),False)
        self.server("tp @s 1.5 200 7.5 180 0")
        self.wait_for(self.player,lambda p:abs(p["position"][2]-7.5)<.05,"second-corner viewpoint")
        self.corner((1,201,4),True)
        self.server("tp @s 0.5 200 0.5 0 0")
        self.wait_for(self.player,lambda p:p["on_ground"] and abs(p["position"][2]-.5)<.05,"starting outside selection")
        time.sleep(.3)
    def corner(self,p,second,ui=False):
        eye=list(self.player()["position"]);eye[1]+=1.62
        dx,dy,dz=[p[i]+.5-eye[i] for i in range(3)]
        self.command(f"look {math.degrees(math.atan2(-dx,dz))} {-math.degrees(math.atan2(dy,math.hypot(dx,dz)))}")
        time.sleep(.15)
        if ui:
            self.key("key.keyboard.right.shift");self.search("area clear");self.click('Select("area_clear")')
            self.click('Choice("clear_second")' if second else 'Choice("clear_first")',pane="settings")
            self.key("key.keyboard.escape")
        else:self.command("rusty-clear-corner "+("second" if second else "first"))
    def begin(self,ui=False):
        if ui:
            self.key("key.keyboard.right.shift");self.search("area clear");self.click('Select("area_clear")');self.shot("area-clear-settings");self.click('Toggle("area_clear")')
        else:self.command("rusty-clear-toggle")
    def ended(self):
        def observed():
            state=self.clear();entry={"state":state,"player":self.player()}
            self.report.setdefault("excavation_trace",[]).append(entry)
            return state
        return self.wait_for(observed,lambda s:not s["active"],"area worker stops",timeout=90)
    def outside(self):
        for p in [(-1,200,3),(2,200,3),(0,199,3),(1,199,4)]:
            b=self.command("block "+" ".join(map(str,p)))
            self.require(("bedrock" if p[1]==199 else "stone") in str(b),"Outside/support block changed: "+str(p))
    def exercise(self):
        self.launch()
        for cmd in ["gamerule send_command_feedback false","gamerule advance_time false","gamerule spawn_mobs false","gamerule keep_inventory true","time set noon","weather clear"]: self.server(cmd)
        self.load_area(-4,-4,5,9,y=210)
        self.phase="explicit selection required"
        rejected=False
        try:self.command("rusty-clear-toggle")
        except AcceptanceFailure:rejected=True
        self.require(rejected and not self.clear()["active"],"Area Clear started without two corners")
        self.passed("No movement or excavation can start without an explicit two-corner selection")
        self.phase="bounded top-down excavation"
        self.fixture();self.begin(ui=True);state=self.ended()
        self.require(state["result"]=="Area cleared" and state["count"]==8,"Area did not fully clear: "+str(state))
        for x in range(2):
            for y in range(200,202):
                for z in range(3,5): self.require("air" in str(self.command(f"block {x} {y} {z}")),"Selected block remains")
        self.outside();self.passed("Two-corner menu starts a full eight-block survival excavation; adjacent identical stone and support stay intact",result=state);self.shot("area-cleared")
        self.phase="ordinary partial-shape blocks"
        self.fixture();self.server("fill 0 200 3 1 201 4 air");self.server("setblock 0 200 3 red_carpet")
        self.wait_for(lambda:self.command("block 0 200 3"),lambda b:"red_carpet" in str(b),"received thin carpet")
        self.begin();state=self.ended()
        self.require(state["result"]=="Area cleared" and state["count"]==1,"Thin block was not aimed at and cleared: "+str(state))
        self.outside();self.passed("Ordinary partial-shape blocks are mined through their actual visible outline",result=state)
        self.phase="actual corner buttons"
        self.fixture();self.corner((0,200,3),False,ui=True)
        self.require("2 × 2 × 2" in self.clear()["selection"],"Menu did not retain the two-corner region")
        self.passed("Actual corner button captures the crosshair block and preserves the inclusive box")
        self.phase="protected container"
        self.server('setblock 0 201 3 chest{Items:[{Slot:0b,id:"minecraft:diamond",count:7}]}')
        self.wait_for(lambda:self.command("block 0 201 3"),lambda b:"chest" in str(b),"received protected chest")
        self.begin();state=self.ended()
        self.require(state["result"]!="Area cleared" and "chest" in str(self.command("block 0 201 3")),"Container was removed or hidden by false completion")
        self.outside();self.passed("Containers survive and protected leftovers are not reported as a cleared area",result=state)
        self.phase="protected named wall sign"
        self.fixture()
        for cmd in ["fill 0 200 3 1 201 4 air","setblock 0 200 3 stone",'setblock 1 200 3 oak_wall_sign[facing=east]{front_text:{messages:["KEEP RUSTY","","",""]}}']:self.server(cmd)
        self.wait_for(lambda:self.command("block 1 200 3"),lambda b:"oak_wall_sign" in str(b),"received supported named wall sign")
        time.sleep(.5);self.begin();state=self.ended()
        self.require(state["count"]==0 and state["result"]!="Area cleared","Protected sign support was excavated")
        self.require("oak_wall_sign" in str(self.command("block 1 200 3")) and "stone" in str(self.command("block 0 200 3")),"Protected sign or its support was removed")
        self.passed("Protected partial objects inside the box preserve their adjacent support",result=state)
        self.phase="outside-border protected sign"
        self.fixture();self.corner((0,200,3),False);self.corner((0,200,3),True)
        for cmd in ["fill 0 200 3 1 201 4 air","setblock 0 200 3 stone",'setblock 1 200 3 oak_wall_sign[facing=east]{front_text:{messages:["KEEP RUSTY","","",""]}}',"worldborder center -1.5 3.5","worldborder set 5","tp @s 0.5 200 2.5 0 0"]:self.server(cmd)
        self.wait_for(self.player,lambda p:p["on_ground"] and abs(p["position"][2]-2.5)<.05,"safe body inside the offset border")
        self.wait_for(lambda:self.command("block 1 200 3"),lambda b:"oak_wall_sign" in str(b),"received protected sign beyond border")
        try:
            self.begin();state=self.ended()
            self.require(state["count"]==0 and state["result"]!="Area cleared","Border route wall hid an outside attachment")
            self.require("stone" in str(self.command("block 0 200 3")) and "oak_wall_sign" in str(self.command("block 1 200 3")),"An outside-border sign or its inside support was destroyed")
            self.passed("Border route constraints preserve received outside attachments and their inside support",result=state)
        finally:
            self.server("worldborder center 0.0 0.0");self.server("worldborder set 59999968");time.sleep(.3)
        self.phase="live world border refusal"
        self.fixture();self.server("worldborder center 0.0 0.0");self.server("worldborder set 8");time.sleep(.5)
        rejected=False
        try:
            try:self.command("rusty-clear-toggle")
            except AcceptanceFailure:rejected=True
            self.require(rejected and not self.clear()["active"],"Selection crossing a shrunken border was accepted")
            self.require("stone" in str(self.command("block 0 200 3")),"Border refusal already excavated a selected block")
            self.passed("A previously captured box crossing a smaller live border is refused before activation")
        finally:self.server("worldborder set 59999968");time.sleep(.3)
        self.phase="world border changes during a run"
        self.fixture();self.begin();self.server("worldborder set 8")
        try:
            state=self.ended();self.require(state["count"]==0 and "border" in str(state["result"]).lower(),"Shrinking border did not cancel the old run")
            self.passed("A changing border cancels the active selection before excavation",result=state)
        finally:self.server("worldborder set 59999968");time.sleep(.3)
        self.phase="fluid protection"
        self.fixture();self.server("setblock 0 201 3 water");time.sleep(.5);self.begin();state=self.ended()
        self.require(state["result"]!="Area cleared" and state["count"]==0,"Fluid danger allowed excavation")
        self.outside();self.passed("Fluid near the working region stops unsafe excavation",result=state)
        self.phase="manual takeover"
        self.fixture();self.begin();self.key("key.keyboard.w","down");state=self.ended();self.key("key.keyboard.w","up");self.command("release")
        self.require(state["count"]==0,"Manual takeover continued clearing")
        time.sleep(.5);self.require(not self.clear()["active"],"Manual stop restarted")
        self.passed("Manual movement cancels Area Clear and never resumes it",result=state)
        self.phase="menu cancellation"
        self.fixture();self.begin();self.key("key.keyboard.right.shift");state=self.ended()
        self.require(state["count"]==0,"Menu retained Area Clear controls");self.key("key.keyboard.escape")
        self.passed("Opening the menu stops Area Clear before an action",result=state)
        self.phase="worn tool"
        self.fixture();self.server("item replace entity @s hotbar.0 with diamond_pickaxe[minecraft:damage=1550]");time.sleep(.3);self.begin();state=self.ended()
        self.require(state["count"]==0 and state["result"]!="Area cleared","Worn tool cleared selected blocks")
        self.passed("Durability reserve protects worn tools without falsely reporting completion",result=state)
        self.phase="dimension-scoped selection"
        self.fixture();self.begin();self.server("execute in minecraft:the_nether run tp @s 0.5 200 0.5 0 0");state=self.ended()
        self.wait_for(self.clear,lambda s:"both corners" in s["selection"],"old-world corners discarded",timeout=30)
        self.passed("Dimension changes stop the old route and discard both corners",result=state)
    def finish(self):
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"area-clear-acceptance.json")

def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ["binary","resources","output"]:p.add_argument("--"+name,required=True,type=Path)
    p.add_argument("--width",default=960,type=int);p.add_argument("--height",default=540,type=int);p.add_argument("--startup-timeout",default=240,type=float)
    args=p.parse_args();smoke=None
    try:
        smoke=AreaClearSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as e:
        if smoke:smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(e)})
        print("FAIL "+str(e),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()
if __name__=="__main__":raise SystemExit(main())
