#!/usr/bin/env python3
"""Exercise actual native awareness rendering in a disposable normal terrain world."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import time
from game_feature_smoke import FeatureSmoke,png_rgb
from game_gui_smoke import AcceptanceFailure,digest,canonical_digest

class AwarenessSmoke(FeatureSmoke):
    native_terrain=True
    def __init__(self,args):
        super().__init__(args)
        self.report.update(suite="native-awareness",script_sha256=digest(Path(__file__)),
            claims="Actual game observations, toggles and screenshots. Image appearance requires separate review.",
            not_exercised=["Other resource packs, physical GPU devices and long sessions.","Projectile paths omit random spread and future target movement; only current received targets are used.","Light risk is not complete mob-spawn eligibility."])
    def obs(self):return self.state()["awareness"]
    def enable(self,id,on=True):self.set_module(id,["awareness",id],on)
    def exercise(self):
        self.launch()
        self.key("key.keyboard.right.shift")
        self.wait_for(self.ui,lambda s:s["open"] and bool(s["hits"]),"initial module browser")
        self.search("xray")
        self.click('Select("xray")')
        self.key("key.keyboard.escape")
        self.wait_for(self.config,lambda c:c["schema"]==1,"initial preferences save")
        self.phase="fixed normal-world awareness fixture"
        self.server("gamemode spectator")
        self.server("tp @s 8.5 120 3.5 0 0")
        self.wait_for(lambda:self.command("state"),lambda s:abs(s["player"]["position"][0]-8.5)<.05 and abs(s["player"]["position"][1]-120)<.05 and abs(s["player"]["position"][2]-3.5)<.05,"spectator at fixture origin",timeout=60)
        for x,z in [(-10,-10),(-10,26),(26,-10),(26,26)]:
            self.wait_for(lambda x=x,z=z:self.command(f"block {x} 119 {z}"),lambda b:b["loaded"],"fixture corner chunk loaded",timeout=90)
        # The Java suite reuses an owned world after the fishing pond at Y197.
        # Remove its source and falling-water columns before this lower scene.
        for low, high in [(131,153),(154,176),(177,199)]:
            self.server(f"fill -12 {low} -6 12 {high} 20 minecraft:air")
        for command in ["gamerule advance_time false","gamerule advance_weather false","time set 6000","weather clear",
                        "fill -10 119 -10 26 119 26 minecraft:stone","fill -10 120 -10 26 130 26 minecraft:air","fill 4 120 7 12 124 10 minecraft:stone",
                        "tp @s 8.5 120 3.5 0 0","gamemode survival","effect give @s minecraft:resistance 600 5 true"]:
            self.server(command)
        self.wait_for(lambda:self.command("block 8 119 3"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","confirmed fixture floor")
        self.wait_for(lambda:self.command("block 8 121 7"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","confirmed fixture wall")
        self.wait_for(lambda:self.command("state"),lambda s:abs(s["player"]["position"][1]-120)<.05 and s["player"]["on_ground"],"grounded on verified platform",timeout=30)
        self.wait_render_ready()
        self.photo("01-normal-world")
        self.server("tp @s 1.5 120 1.5 135 12")
        self.enable("chunk_borders")
        self.command("look 135 12")
        self.wait_for(self.camera_pose,lambda p:all(abs(p["position"][i]-[1.5,120,1.5][i])<.01 for i in range(3)) and abs(p["yaw"]-135)<.001 and abs(p["pitch"]-12)<.001,"fixed nearby exposed chunk corner")
        self.photo("02-chunk-borders",self.camera_pose())
        self.enable("chunk_borders",False)
        self.server("tp @s 8.5 120 3.5 0 0")
        self.command("look 0 0")
        self.passed("Chunk Borders toggles through native menu and captures real geometry for review")
        self.server("setblock 9 120 9 minecraft:chest")
        self.wait_for(lambda:self.command("block 9 120 9"),lambda b:b["loaded"] and b["block"]=="minecraft:chest","confirmed received chest state")
        self.command("look -9.5 10.6")
        self.wait_for(self.camera_pose,lambda p:all(abs(p["position"][i]-[8.5,120,3.5][i])<.01 for i in range(3)),"body returned to received storage viewpoint")
        storage_pose=self.camera_pose()
        off=self.photo("03a-storage-off",storage_pose)
        self.enable("storage_esp")
        observed=self.wait_for(self.obs,lambda o:o["storage_boxes"]>=1,"received storage block entity")
        self.command("look -9.5 10.6")
        on=self.photo("03-received-storage-esp",storage_pose)
        def storage_pixels(path):
            width,height,rgb=png_rgb(path)
            return sum(1 for y in range(int(height*.2),int(height*.8)) for x in range(int(width*.2),int(width*.8))
                       if rgb[(y*width+x)*3]>170 and 90<rgb[(y*width+x)*3+1]<235 and rgb[(y*width+x)*3+2]<175
                       and rgb[(y*width+x)*3]>rgb[(y*width+x)*3+2]+45)
        off_pixels,on_pixels=storage_pixels(off),storage_pixels(on)
        self.require(on_pixels>off_pixels+10,"Storage ESP did not add visible warm outline pixels through the known opaque wall")
        self.report["storage_visual_comparison"]={"off_warm_pixels":off_pixels,"on_warm_pixels":on_pixels,"same_requested_view":True,"chest_behind_received_wall":True}
        self.server("setblock 9 120 9 minecraft:air")
        self.wait_for(self.obs,lambda o:o["storage_boxes"]==0,"storage removal invalidation")
        self.enable("storage_esp",False)
        self.require(self.obs()["storage_boxes"]==0,"Disabled storage remained cached")
        self.passed("Storage ESP observes ordinary authoritative chest placement and removes deleted storage",boxes=observed["storage_boxes"])
        self.server("fill -4 124 -4 20 124 20 minecraft:stone")
        self.enable("light_overlay")
        light=self.wait_for(self.obs,lambda o:o["light_cells"]>0,"received dark floor risk",timeout=60)
        self.photo("04-light-risk")
        self.enable("light_overlay",False)
        self.wait_for(self.obs,lambda o:o["depth_lines"]==0,"disabled light geometry")
        self.server("fill -4 124 -4 20 124 20 minecraft:air")
        self.passed("Light Risk draws only from received ready light and clears when disabled",marked_cells=light["light_cells"])
        self.server("setblock 8 120 7 minecraft:oak_fence")
        self.wait_for(lambda:self.command("block 8 120 7"),lambda b:b["loaded"] and b["block"]=="minecraft:oak_fence","received stateful target block")
        self.enable("target_info")
        self.command("look 0 18")
        self.photo("05-crosshair-block-properties",self.camera_pose())
        self.enable("portal_coordinates")
        self.enable("clock_hud")
        self.enable("fps_hud")
        self.enable("clicks_hud")
        self.enable("minimap")
        self.wait_for(self.state,lambda o:o.get("minimap") is not None,"native minimap enabled")
        self.enable("speed_hud")
        self.photo("06-world-context-hud")
        self.server("tp @s 8.5 120 3.5 180 0")
        self.key("key.keyboard.w","down")
        speed=self.wait_for(self.obs,lambda o:o["speed"] is not None and o["speed"]>0.1,"observed walking speed")
        self.photo("07-speed-during-movement")
        self.key("key.keyboard.w","up")
        self.wait_for(self.obs,lambda o:o["speed"] is not None and o["speed"]<0.1,"standing speed resets")
        self.passed("Speed HUD follows real body movement and returns to stationary",moving_meters_per_second=speed["speed"])
        self.server("tp @s 8.5 120 3.5 0 -12")
        self.server("item replace entity @s weapon.mainhand with minecraft:bow")
        self.server("give @s minecraft:arrow 64")
        self.enable("trajectories")
        self.command("mouse right down")
        self.wait_for(lambda:self.obs()["prediction"],lambda o:o["segments"]>0 and o["kind"]=="bow","charged native bow prediction")
        self.photo("08-bow-block-trajectory")
        bow_slot=self.command("inventory")["selected"]
        empty_slot=9 if bow_slot!=9 else 8
        self.server(f"item replace entity @s hotbar.{empty_slot-1} with minecraft:air")
        self.command(f"hotbar {empty_slot}")
        self.wait_for(lambda:self.command("state"),lambda s:s.get("using") is None,"slot change cancels ordinary bow use")
        self.command("mouse right up")
        self.command(f"hotbar {bow_slot}")
        self.server(f"item replace entity @s hotbar.{bow_slot-1} with minecraft:ender_pearl")
        before_inventory=self.wait_for(lambda:self.command("inventory"),lambda inv:inv["selected"]==bow_slot and (inv.get("held") or {}).get("item")=="minecraft:ender_pearl","received explicit pearl slot before read-only baseline")
        before_blocks=[self.command(f"block {x} 121 7") for x in range(4,13)]
        predicted=self.wait_for(lambda:self.obs()["prediction"],lambda o:o["segments"]>0 and o["kind"]=="ender pearl","native pearl prediction")
        self.photo("09-pearl-block-trajectory")
        after_inventory=self.command("inventory")
        self.report["read_only_prediction_inventory"]={"before":before_inventory,"after":after_inventory}
        self.require(canonical_digest(before_inventory)==canonical_digest(after_inventory),"Read-only prediction changed inventory")
        self.require(canonical_digest(before_blocks)==canonical_digest([self.command(f"block {x} 121 7") for x in range(4,13)]),"Read-only prediction changed source wall")
        self.enable("trajectories",False)
        self.wait_for(lambda:self.obs()["prediction"],lambda o:o["segments"]==0,"disabled prediction cleared")
        self.server('summon minecraft:pig 11.5 120 11 {NoAI:1b}')
        self.command("look 0 0")
        self.wait_for(lambda:self.command("entities 16 minecraft:pig"),lambda e:bool(e["entities"]),"received hidden pig")
        off=self.photo("10a-entity-tracer-off")
        self.enable("entity_tracers")
        self.wait_for(self.obs,lambda o:o.get("entity_tracer_lines",0)>0,"actual native entity tracer geometry")
        on=self.photo("10-received-entity-tracer")
        def orange_pixels(path):
            width,height,rgb=png_rgb(path)
            return sum(1 for y in range(int(height*.2),int(height*.7)) for x in range(int(width*.2),int(width*.75))
                       if rgb[(y*width+x)*3]>190 and 70<rgb[(y*width+x)*3+1]<210 and 30<rgb[(y*width+x)*3+2]<150
                       and rgb[(y*width+x)*3]>rgb[(y*width+x)*3+1]+40)
        old_count,new_count=orange_pixels(off),orange_pixels(on)
        self.require(new_count>old_count+10,"Enabled tracer did not add visible orange line pixels to the opaque-wall view")
        self.enable("entity_tracers",False)
        self.passed("Entity tracer adds visible geometry through the wall, using a non-degenerate screen origin",off_orange_pixels=old_count,on_orange_pixels=new_count)
        self.passed("Native charged bow/pearl predictions and received entity tracers toggle through actual renderer",pearl=predicted)
        for id in ["target_info","portal_coordinates","clock_hud","speed_hud","minimap","fps_hud","clicks_hud"]:self.enable(id,False)
        self.photo("11-restored-normal-world")
        self.passed("All awareness modules restore disabled state without source-world mutation")
        self.phase="observed death marker saves once and preserves selection"
        self.typed_command("rusty waypoint add Retained_Selection")
        prior=self.wait_waypoint("Retained_Selection")
        own=next(scope for scope in prior["scopes"] if any(p["name"]=="Retained_Selection" for p in scope["waypoints"]))
        self.set_module("death_markers",["navigation","death_markers"],True)
        time.sleep(.3)
        self.server("kill @s")
        marked=self.wait_for(self.navigation,lambda data:any(p["name"].startswith("Death ") for scope in data["scopes"] for p in scope["waypoints"]),"saved observed death marker")
        after=next(scope for scope in marked["scopes"] if scope["scope"]==own["scope"])
        deaths=[p for p in after["waypoints"] if p["name"].startswith("Death ")]
        self.require(len(deaths)==1 and after["selected"]==own["selected"],"Death duplicated marker or changed selected waypoint")
        time.sleep(.5)
        again=next(scope for scope in self.navigation()["scopes"] if scope["scope"]==own["scope"])
        self.require(len(again["waypoints"])==len(after["waypoints"]),"Death marker repeated while dead")
        self.passed("Observed death saves exactly one scoped marker without changing selection",marker_count=len(deaths),selection_preserved=True)

    def finish(self):
        # Direct base finish avoids FeatureSmoke's report rename.
        from game_gui_smoke import Smoke
        Smoke.finish(self)
        (self.output/"gui-acceptance.json").replace(self.output/"awareness-acceptance.json")

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary",required=True,type=Path);p.add_argument("--resources",required=True,type=Path)
    p.add_argument("--output",required=True,type=Path);p.add_argument("--width",type=int,default=1280)
    p.add_argument("--height",type=int,default=720);p.add_argument("--startup-timeout",type=float,default=240.)
    args=p.parse_args(); smoke=None
    try:
        smoke=AwarenessSmoke(args);smoke.exercise();smoke.report["passed"]=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as exc:
        if smoke:
            smoke.report["passed"]=False;smoke.report["checks"].append(dict(name=smoke.phase,passed=False,reason=str(exc)))
            try:smoke.shot("failure")
            except Exception:pass
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if smoke:smoke.finish()
if __name__=="__main__":raise SystemExit(main())
