#!/usr/bin/env python3
"""Exercise real GUI capture and gameplay routing for 38 persisted module flags.

Only a new private, empty Creative fixture is used. Four special modules have
separate behavior/input suites; this is not a hardware-device coverage claim.
"""
from __future__ import annotations
import argparse
from pathlib import Path
import re
import subprocess
import sys
import time
from game_inventory_smoke import InventorySmoke
from game_gui_smoke import Smoke, AcceptanceFailure, digest, canonical_digest

FLAG_PATHS={
    "finder":("finder_enabled",),"entity_esp":("entity_esp",),"fullbright":("fullbright",),
    "no_fog":("no_fog",),"clear_weather":("clear_weather",),"coordinates":("coords_hud",),
    "auto_sprint":("auto_sprint",),"no_hurt_camera":("no_hurt_camera",),"status_hud":("hud",),
    "compass":("navigation","compass_enabled"),"waypoints":("navigation","waypoint_hud"),
    "breadcrumbs":("navigation","breadcrumbs_enabled"),"death_markers":("navigation","death_markers"),
    "equipment_hud":("utilities","equipment_hud"),"environment_hud":("utilities","environment_hud"),
    "container_preview":("utilities","container_preview"),"build_planner":("build","enabled"),
    **{key:("awareness",key) for key in ["chunk_borders","light_overlay","storage_esp","trajectories",
        "target_info","portal_coordinates","speed_hud","clock_hud","entity_tracers","minimap","fps_hud","clicks_hud"]},
    **{key:("assistance",key) for key in ["safe_walk","auto_jump","auto_tool","trigger_bot","auto_eat",
        "auto_fish","auto_totem","armor_assist","refill"]},
}
SPECIAL={"xray":"X/Y actual rebind, hold/toggle and received-world rendering fixtures",
    "zoom":"Actual held C view change/release restoration fixture",
    "freecam":"Actual U detached view/body preservation fixture",
    "waypoint_walk":"Transient task command/menu and cancellation fixtures; not a persisted flag"}


def catalog_modules():
    source=Path(__file__).resolve().parents[1]/"mixins/rusty/catalog.rs"
    found=re.findall(r'ModuleSpec \{\s*id: "([^"]+)",\s*name: "([^"]+)".*?default_key: Some\("([^"]+)"\)',source.read_text(encoding="utf-8"),re.S)
    modules={module:(name,default) for module,name,default in found}
    if len(found)!=42 or len(modules)!=42 or set(modules)!=(set(FLAG_PATHS)|set(SPECIAL)):
        raise AcceptanceFailure("Catalog no longer matches the38 persisted flags plus4 explicit special modules")
    return modules,source


class BindingSmoke(InventorySmoke):
    def __init__(self,args):
        self.binding_budget_active=False
        super().__init__(args)
        self.modules,catalog=catalog_modules()
        self.report.update(suite="native-module-binding-routing",script_sha256=digest(Path(__file__)),
            catalog_sha256=digest(catalog),tested_module_count=38,excluded_special_modules=SPECIAL,
            claims="Actual native GUI F8 capture/save, ordinary gameplay press routing to each intended flag, restoration and per-module reset in an empty private Creative world.",
            not_exercised=["These toggles do not accept the underlying38 gameplay/rendering implementations; use their behavior and image fixtures.","Physical keyboard/controller hardware, arbitrary remappings and optional shortcuts for the4 special modules are not individually covered by this loop."])
        self.report["binding_rows"]=[]

    def command(self,*args,**kwargs):
        if self.binding_budget_active and time.monotonic()>self.binding_deadline:
            raise AcceptanceFailure("Binding fixture exceeded its 180-second post-launch budget")
        return super().command(*args,**kwargs)

    def flag(self,module,config=None):
        value=self.config() if config is None else config
        for part in FLAG_PATHS[module]:value=value[part]
        self.require(type(value) is bool,"Expected a persisted bool for "+module)
        return value

    def browser(self,module):
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"binding browser")
        self.search(self.modules[module][0]);self.click(f'Select("{module}")')

    def close_browser(self):
        self.key("key.keyboard.escape")
        self.wait_for(self.ui,lambda s:not s["open"],"closed binding browser")
        first=self.wait_for(lambda:self.command("status"),lambda s:s["screen"]=="playing","Playing after capture")
        self.wait_for(lambda:self.command("status"),lambda s:s["frames"]>=first["frames"]+2,"fresh gameplay frames after capture",timeout=5)

    def f8_owners(self):return [key for key,value in self.bindings().items() if value=="key.keyboard.f8"]

    def invariants(self,body,inventory):
        cfg=self.config()
        self.require(not any(cfg["assistance"][key] for key in cfg["assistance"] if key in FLAG_PATHS),"An action-module default was not restored")
        self.require(not cfg["build"]["enabled"],"Build Planner remained enabled")
        self.require(canonical_digest(self.command("inventory"))==inventory,"Binding routing changed the empty fixture inventory")
        current=self.command("state")
        self.require(current["gamemode"]=="creative","Binding fixture left Creative mode")
        self.require(all(abs(a-b)<.01 for a,b in zip(body["position"],current["player"]["position"])),"Binding routing moved the grounded Creative body")
        self.require(abs(body["yaw"]-current["player"]["yaw"])<.01 and abs(body["pitch"]-current["player"]["pitch"])<.01,"Binding routing rotated the fixture body")
        self.require(body["health"]==current["player"]["health"] and body["food"]==current["player"]["food"],"Binding routing changed health/food")

    def exercise(self):
        self.launch();self.binding_deadline=time.monotonic()+180;self.binding_budget_active=True
        self.phase="fresh empty Creative binding fixture"
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"initial binding browser")
        self.search("auto eat");self.click('Select("auto_eat")')
        self.wait_for(self.config,lambda c:"auto_eat" in c["assistance"],"persisted initial settings")
        self.key("key.keyboard.escape");self.off();self.setup()
        for command in ["gamerule spawn_mobs false","gamerule advance_time false","time set noon",
                        "kill @e[type=!minecraft:player]","kill @e[type=minecraft:item]","clear @s","gamemode creative @s"]:
            self.server(command)
        self.command("release")
        self.wait_for(lambda:self.command("state"),lambda s:s["gamemode"]=="creative","received Creative mode")
        self.wait_for(self.player,lambda p:p["on_ground"] and abs(p["position"][1]-200)<.01,"settled empty Creative body")
        current=self.command("state")
        body={key:current["player"][key] for key in ("position","yaw","pitch","health","food")}
        inventory=canonical_digest(self.command("inventory"))
        self.require(not self.command("inventory")["counts"],"Creative binding fixture is not empty")
        self.require(not self.f8_owners(),"Fresh profile unexpectedly already owns F8")
        self.invariants(body,inventory)
        for module in FLAG_PATHS:
            self.phase="actual F8 routing: "+module
            default=self.modules[module][1];binding="key.rusty."+module
            original=self.bindings().get(binding,default);before=self.flag(module)
            self.require(original==default,"Fresh fixture has a non-default module binding: "+module)
            self.browser(module);self.click(f'Bind("{module}")',pane="detail")
            self.wait_for(self.ui,lambda s:s["capture"]==module,"actual key capture "+module)
            self.key("key.keyboard.f8")
            self.wait_for(self.bindings,lambda b:b.get(binding)=="key.keyboard.f8","saved Horizon F8 binding "+module)
            self.require(self.flag(module)==before,"Captured F8 leaked into an action for "+module)
            self.require(self.f8_owners()==[binding],"Duplicate temporary F8 ownership")
            self.close_browser();self.key("key.keyboard.f8")
            self.wait_for(self.config,lambda c:self.flag(module,c)!=before,"fresh F8 toggles "+module)
            first=self.flag(module)
            self.key("key.keyboard.f8")
            self.wait_for(self.config,lambda c:self.flag(module,c)==before,"fresh F8 restores "+module)
            self.invariants(body,inventory)
            self.browser(module);self.click(f'Bind("{module}")',right=True,pane="detail")
            self.wait_for(self.bindings,lambda b:b.get(binding,default)==original,"restored original binding "+module)
            self.require(not self.f8_owners(),"Temporary F8 remains after reset: "+module)
            self.close_browser()
            self.report["binding_rows"].append({"module":module,"flag_path":list(FLAG_PATHS[module]),"original_binding":original,
                "capture_binding":"key.keyboard.f8","flag_before":before,"flag_after_first_press":first,
                "flag_after_second_press":self.flag(module),"restored_binding":self.bindings().get(binding,default),"passed":True})
            print("PASS native F8 binding route "+module,flush=True)
        self.invariants(body,inventory)
        self.passed("All 38 persisted module flags respond to actual captured gameplay bindings and restore cleanly",modules=38,no_leftover_f8=not self.f8_owners(),body=body,inventory_unchanged=True)
        self.report["post_launch_elapsed_seconds"]=round(180-(self.binding_deadline-time.monotonic()),3)

    def finish(self):
        self.binding_budget_active=False;Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"binding-acceptance.json")


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for key in ["binary","resources","output"]:p.add_argument("--"+key,required=True,type=Path)
    p.add_argument("--width",type=int,default=1280);p.add_argument("--height",type=int,default=720);p.add_argument("--startup-timeout",type=float,default=300)
    args=p.parse_args();args.cases=[];smoke=None
    try:smoke=BindingSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if smoke:smoke.report["passed"]=False;smoke.report["checks"].append({"name":smoke.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()

if __name__=="__main__":raise SystemExit(main())
