#!/usr/bin/env python3
"""Conservative empty-slot armor/refill checks in a disposable native world.

Predicted client slots are not described as acknowledgments. The independent
Java fixture also queries authoritative Inventory NBT after these cases.
"""
from __future__ import annotations
import argparse
from pathlib import Path
import subprocess
import sys
import time
from game_automation_smoke import AutomationSmoke
from game_gui_smoke import Smoke, AcceptanceFailure, digest


class InventorySmoke(AutomationSmoke):
    def __init__(self,args):
        super().__init__(args)
        self.report.update(suite="native-conservative-inventory",script_sha256=digest(Path(__file__)),
            claims="Ordinary inventory actions, received/predicted slot observations and preservation checks in a fresh controlled world.",
            not_exercised=["Separate Java authority is asserted by the Java fixture, not this integrated fixture.","High latency, correction rejection and arbitrary component overrides are not covered here."])

    def off(self):
        super().off()
        for key in ["armor_assist","refill"]:
            if self.config()["assistance"].get(key,False):self.set_module(key,["assistance",key],False)

    def armor(self):
        self.setup()
        for text in [
            'item replace entity @s inventory.0 with minecraft:diamond_helmet[minecraft:enchantments={"minecraft:binding_curse":1}]',
            "item replace entity @s inventory.1 with minecraft:iron_helmet",
            "item replace entity @s inventory.2 with minecraft:iron_chestplate",
            "item replace entity @s inventory.3 with minecraft:diamond_boots",
            "item replace entity @s armor.feet with minecraft:leather_boots"]:self.server(text)
        initial=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["main"][0].get("item")=="minecraft:diamond_helmet" and inv["armor"][3].get("item")=="minecraft:leather_boots","received armor sources and protected worn boots")
        self.set_module("armor_assist",["assistance","armor_assist"])
        equipped=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["armor"][0].get("item")=="minecraft:iron_helmet" and inv["armor"][1].get("item")=="minecraft:iron_chestplate","ordinary quick moves equip empty head and chest",timeout=10)
        time.sleep(1)
        final=self.command("inventory")
        self.require(final["main"][0].get("item")=="minecraft:diamond_helmet","Armor Assist equipped a binding-cursed helmet")
        self.require(final["armor"][3].get("item")=="minecraft:leather_boots" and final["main"][3].get("item")=="minecraft:diamond_boots","Armor Assist replaced existing boots")
        self.require(final["main"][1].get("item") is None and final["main"][2].get("item") is None and final["carried"] is None,"Armor click left a duplicate source or carried cursor stack")
        # Explicit positive received equipment state, followed by break/removal.
        # No menu, focus transition or module toggle rearms this second fill.
        self.server("item replace entity @s armor.head with minecraft:iron_helmet[minecraft:damage=1]")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv["armor"][0].get("damage")==1,"actual positive head equipment update")
        self.server("item replace entity @s inventory.4 with minecraft:iron_helmet")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv["main"][4].get("item")=="minecraft:iron_helmet","received replacement helmet source")
        self.server("item replace entity @s armor.head with minecraft:air")
        repeated=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["armor"][0].get("item")=="minecraft:iron_helmet" and inv["main"][4].get("item") is None,"second empty head fill after received equipment loss",timeout=10)
        self.passed("Armor Assist fills empty parts, rejects binding curse, preserves existing equipment and rearms after real gear loss",initial=initial,equipped=equipped,final=final,repeated=repeated)

    def refill_single(self, resume=None, controller=False):
        self.setup()
        if controller:
            self.command("pad release")
            self.require(self.command("pad status")["enabled"],"Controller fixture requires the fresh default enabled controller settings")
        def use_click():
            if controller:
                self.command("pad trigger left 1")
                try:
                    self.wait_for(lambda:self.command("pad status"),lambda s:"lt" in s["held"],"analog controller USE edge",timeout=3)
                    time.sleep(.08)
                finally:self.command("pad trigger left 0")
                self.wait_for(lambda:self.command("pad status"),lambda s:"lt" not in s["held"],"ordinary controller USE released after its tick",timeout=3)
            else:self.command("mouse right click")
        for slot in ["hotbar.0","inventory.0","inventory.1"]:
            self.server(f"item replace entity @s {slot} with minecraft:stone 1")
        self.wait_for(lambda:self.command("inventory"),lambda inv:all(row.get("count")==1 for row in [inv["hotbar"][0],inv["main"][0],inv["main"][1]]),"three received separate single-item stacks")
        self.command("hotbar 1")
        binding=None
        if resume=="reenable":
            self.set_module("armor_assist",["assistance","armor_assist"])
            self.key("key.keyboard.right.shift")
            self.wait_for(self.ui,lambda state:state["open"] and bool(state["hits"]),"independent refill keybind browser")
            self.search("auto refill");self.click('Select("refill")')
            self.click('Bind("refill")',pane="detail")
            self.wait_for(self.ui,lambda state:state["capture"]=="refill","ordinary refill key capture")
            self.key("key.keyboard.r")
            binding=self.wait_for(self.bindings,lambda b:b.get("key.rusty.refill")=="key.keyboard.r","persisted real Refill R binding")
            self.require(not self.config()["assistance"]["refill"],"Captured R leaked into a module action")
            self.key("key.keyboard.escape")
            self.wait_for(self.ui,lambda state:not state["open"],"closed refill key capture browser")
            time.sleep(.6)
            self.key("key.keyboard.r")
            self.wait_for(self.config,lambda cfg:cfg["assistance"]["refill"] and cfg["assistance"]["armor_assist"],"real gameplay R enables Refill independently")
        else:self.set_module("refill",["assistance","refill"])
        time.sleep(.9)
        context_before=self.command("rusty-status")
        position_before=self.player()["position"]
        self.command("look 0 60");use_click()
        self.wait_for(lambda:self.command("block 0 200 1"),lambda b:b["block"]=="minecraft:stone","first ordinary single-item placement before any refill claim")
        first=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["hotbar"][0].get("count")==1 and inv["main"][0].get("item") is None,"first single-item reserve refill",timeout=8)
        self.wait_for(lambda:self.command("rusty-status")["inventory_assist"],lambda s:not s["pending"],"first request quiet interval complete")
        if resume=="menu":
            self.key("key.keyboard.right.shift")
            self.wait_for(self.ui,lambda state:state["open"],"inventory intent suspension menu")
            time.sleep(.3);self.key("key.keyboard.escape")
            self.wait_for(self.ui,lambda state:not state["open"],"inventory intent resume")
            time.sleep(.6)
        elif resume=="held_menu":
            self.command("look 0 -60");self.command("mouse right down")
            time.sleep(.2)
            self.key("key.keyboard.right.shift")
            self.wait_for(self.ui,lambda state:state["open"],"menu opened while ordinary USE was held")
            # The host's mouse-up CLI is Playing-only. A blank menu click
            # routes a redundant Down plus the required Released through the
            # actual window-event handler without a gameplay USE invocation.
            self.command("click-at 1 1 right")
            self.require(self.ui()["open"],"Blank corner release closed or activated the menu")
            self.key("key.keyboard.escape")
            self.wait_for(self.ui,lambda state:not state["open"],"closed menu after actual in-menu release event")
            time.sleep(.6)
        elif resume=="held_controller_menu":
            self.command("look 0 -60");self.command("pad trigger left 1")
            self.wait_for(lambda:self.command("pad status"),lambda s:"lt" in s["held"],"controller USE held before menu",timeout=3)
            self.key("key.keyboard.right.shift")
            self.wait_for(self.ui,lambda state:state["open"],"menu opened while controller USE held")
            self.command("pad trigger left 0")
            self.wait_for(lambda:self.command("pad status"),lambda s:"lt" not in s["held"],"controller USE release processed inside menu",timeout=3)
            self.require(self.ui()["open"],"Controller release unexpectedly closed the menu")
            self.key("key.keyboard.escape")
            self.wait_for(self.ui,lambda state:not state["open"],"closed menu after controller USE release")
            time.sleep(.6)
        elif resume=="reenable":
            self.require(not self.ui()["open"],"Refill re-enable test entered a menu")
            self.key("key.keyboard.r")
            self.wait_for(self.config,lambda cfg:not cfg["assistance"]["refill"] and cfg["assistance"]["armor_assist"],"gameplay R disables only Refill")
            self.require(not self.ui()["open"],"Disabling Refill opened a menu")
            self.key("key.keyboard.r")
            self.wait_for(self.config,lambda cfg:cfg["assistance"]["refill"] and cfg["assistance"]["armor_assist"],"gameplay R re-enables only Refill")
            time.sleep(.6)
            before_use=self.command("inventory")
            self.require(before_use["hotbar"][1].get("item") is None,"Re-enable filled an initially empty slot")
        before_second=self.command("rusty-status")["inventory_assist"]
        self.require(before_second["last_action"] is not None and self.config()["assistance"]["refill"] and not self.ui()["open"],"Single-item refill context reset or module/menu state changed before the second use")
        # Keep the context and body stationary: teleports, menus and module
        # toggles could reset the intent guard and hide a repeated-use defect.
        self.command("look 90 60");use_click()
        self.wait_for(lambda:self.command("block -1 200 0"),lambda b:b["block"]=="minecraft:stone","second ordinary single-item placement")
        second=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["hotbar"][0].get("count")==1 and inv["main"][1].get("item") is None,"second single-item reserve refill without menu or toggle",timeout=8)
        position_after=self.player()["position"]
        self.require(all(abs(before-after)<.01 for before,after in zip(position_before,position_after)),"Refill single-item test body/context changed between uses")
        self.passed("Auto Refill replenishes repeated single-item exhaustion"+(" after "+resume if resume else " without context reset"),first=first,second=second,position_before=position_before,position_after=position_after,context_before=context_before,context_after=self.command("rusty-status"),before_second=before_second,resume=resume,binding=binding,controller=controller,telemetry_limit="This build has no context epoch counter; stationary server/client body and retained action state are asserted; resume variants deliberately change only their named menu/module state. Virtual controller injection exercises the ordinary pad route, not physical hardware.")

    def refill_resume(self):self.refill_single(resume="menu")

    def refill_reenable(self):self.refill_single(resume="reenable")

    def refill_held_resume(self):self.refill_single(resume="held_menu")

    def refill_controller(self):self.refill_single(controller=True)

    def refill_controller_held_resume(self):self.refill_single(resume="held_controller_menu",controller=True)

    def refill_tool(self,controller=False):
        self.setup()
        if controller:
            self.command("pad release")
            self.require(self.command("pad status")["enabled"],"Controller tool fixture requires enabled defaults")
        # Vanilla26.3 names this rule block_drops (SDK save.rs GAME_RULES).
        # A mined cobblestone pickup correctly owns a newly empty target, so
        # suppress only this disposable course's block loot during the test.
        self.server("gamerule block_drops false")
        try:self.refill_tool_without_drops(controller=controller)
        finally:self.server("gamerule block_drops true")

    def refill_tool_without_drops(self,controller=False):
        for position in ["0 200 2","-2 200 0"]:
            self.server(f"setblock {position} stone")
            self.wait_for(lambda p=position:self.command("block "+p),lambda b:b["block"]=="minecraft:stone","received last-durability mining target")
        for slot in ["hotbar.0","inventory.0","inventory.1"]:
            self.server(f"item replace entity @s {slot} with minecraft:wooden_pickaxe[minecraft:damage=58]")
        self.wait_for(lambda:self.command("inventory"),lambda inv:all(row.get("item")=="minecraft:wooden_pickaxe" and row.get("damage")==58 for row in [inv["hotbar"][0],inv["main"][0],inv["main"][1]]),"three exact last-durability wooden pickaxes")
        self.command("hotbar 1");self.set_module("refill",["assistance","refill"]);time.sleep(.9)
        position_before=self.player()["position"]
        observations=[]
        for yaw,position,source in [(0,"0 200 2",0),(90,"-2 200 0",1)]:
            self.command(f"look {yaw} 30");self.command("pad trigger right 1" if controller else "mouse left down")
            try:
                if controller:self.wait_for(lambda:self.command("pad status"),lambda s:"rt" in s["held"],"analog controller ATTACK edge",timeout=3)
                self.wait_for(lambda p=position:self.command("block "+p),lambda b:b["block"]=="minecraft:air","ordinary mining breaks the last-durability pickaxe",timeout=8)
            finally:self.command("pad trigger right 0" if controller else "mouse left up")
            if controller:self.wait_for(lambda:self.command("pad status"),lambda s:"rt" not in s["held"],"ordinary controller ATTACK released",timeout=3)
            state=self.wait_for(lambda:self.command("inventory"),lambda inv,s=source:inv["hotbar"][0].get("item")=="minecraft:wooden_pickaxe" and inv["hotbar"][0].get("damage")==58 and inv["main"][s].get("item") is None,"exact replacement after a fresh ordinary attack",timeout=8)
            self.wait_for(lambda:self.command("rusty-status")["inventory_assist"],lambda s:not s["pending"],"tool replacement quiet interval")
            observations.append(state)
        self.require(all(abs(before-after)<.01 for before,after in zip(position_before,self.player()["position"])),"Tool refill body moved/context changed")
        self.passed("Auto Refill replenishes successive broken tools after fresh ordinary attacks",observations=observations,position=position_before,controller=controller)

    def refill_controller_tool(self):self.refill_tool(controller=True)

    def refill(self):
        self.setup()
        for text in ["item replace entity @s hotbar.0 with minecraft:stone 1",
            "item replace entity @s inventory.0 with minecraft:stone[minecraft:custom_name={text:'Named'}] 12",
            "item replace entity @s inventory.1 with minecraft:stone 16"]:self.server(text)
        initial=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["hotbar"][0].get("count")==1 and inv["main"][0].get("count")==12 and inv["main"][1].get("count")==16,"received component-different and exact refill sources")
        self.require("components" in initial["main"][0],"Named stone mismatch fixture did not retain components")
        self.command("hotbar 1")
        self.set_module("refill",["assistance","refill"])
        time.sleep(.9)
        self.command("look 0 60")
        self.command("mouse right click")
        self.wait_for(lambda:self.command("block 0 200 1"),lambda b:b["block"]=="minecraft:stone","ordinary user placement exhausts the original hotbar stack")
        refilled=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["hotbar"][0].get("count")==16 and inv["main"][1].get("item") is None,"single ordinary exact-component refill",timeout=8)
        self.require(refilled["main"][0].get("count")==12 and refilled["hotbar"][1].get("item") is None and refilled["carried"] is None,"Refill used the named source, filled an initially empty slot, or retained a cursor item")
        # A subsequent real server update rearms the owned slot. Only a named
        # mismatch remains, and must never be used as a same-ID substitute.
        self.server("item replace entity @s hotbar.0 with minecraft:stone 15")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv["hotbar"][0].get("count")==15,"received positive restock update")
        self.server("item replace entity @s hotbar.0 with minecraft:air")
        self.wait_for(lambda:self.command("inventory"),lambda inv:inv["hotbar"][0].get("item") is None,"received empty target with only named mismatching source")
        time.sleep(1.2)
        final=self.command("inventory")
        self.require(final["hotbar"][0].get("item") is None and final["main"][0].get("count")==12,"Refill ignored component identity")
        self.passed("Auto Refill restores only a consumed exact-component hotbar stack and preserves named alternatives",initial=initial,refilled=refilled,final=final)

    def exercise(self):
        self.launch();self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda state:state["open"] and bool(state["hits"]),"initial inventory browser")
        self.search("armor assist");self.click('Select("armor_assist")')
        self.wait_for(self.config,lambda cfg:"armor_assist" in cfg["assistance"],"persisted inventory profile")
        self.key("key.keyboard.escape")
        for name,case in [("armor_assist",self.armor),("refill",self.refill),("refill_single",self.refill_single),("refill_tool",self.refill_tool),("refill_resume",self.refill_resume),("refill_reenable",self.refill_reenable),("refill_held_resume",self.refill_held_resume),("refill_controller",self.refill_controller),("refill_controller_held_resume",self.refill_controller_held_resume),("refill_controller_tool",self.refill_controller_tool)]:
            if name not in self.args.cases:continue
            self.phase=name;self.off();case();self.command("release")
        self.off()

    def finish(self):
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"inventory-acceptance.json")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ["binary","resources","output"]:parser.add_argument("--"+name,required=True,type=Path)
    parser.add_argument("--width",default=960,type=int);parser.add_argument("--height",default=540,type=int)
    parser.add_argument("--startup-timeout",default=300,type=float)
    parser.add_argument("--cases",nargs="+",choices=["armor_assist","refill","refill_single","refill_tool","refill_resume","refill_reenable","refill_held_resume","refill_controller","refill_controller_held_resume","refill_controller_tool"],default=["armor_assist","refill","refill_single","refill_tool","refill_resume","refill_reenable","refill_held_resume","refill_controller","refill_controller_held_resume","refill_controller_tool"])
    args=parser.parse_args();smoke=None
    try:
        smoke=InventorySmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()

if __name__=="__main__":raise SystemExit(main())
