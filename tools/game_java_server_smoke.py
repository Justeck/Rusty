#!/usr/bin/env python3
"""Offline loopback Java26.3 server compatibility, in new private directories.

Supply your existing official server JAR and Java25 runtime. Neither is bundled
or downloaded. A single owned native client joins the owned vanilla server;
all authoritative fixture changes go through its ordinary stdin console.
"""
from __future__ import annotations

import argparse
from collections import deque
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import threading
import time
import zipfile

from game_assistance_smoke import AssistanceSmoke
from game_automation_smoke import AutomationSmoke
from game_awareness_smoke import AwarenessSmoke
from game_builder_smoke import BuilderSmoke
from game_feature_smoke import FeatureSmoke
from game_gui_smoke import AcceptanceFailure, Smoke, canonical_digest, digest
from game_inventory_smoke import InventorySmoke
from game_multiplayer_smoke import Peer, GUEST_NAME

SERVER_SHA1 = "33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c"


class JavaServer:
    def __init__(self, args, output):
        self.args = args
        self.directory = output / "private-java-server"
        self.directory.mkdir()
        for name in ["home", "temp", "appdata", "localappdata"]:
            (self.directory / name).mkdir()
        self.lines = deque(maxlen=10000)
        self.serial = 0
        self.marker = 0
        self.process = None
        self.log = None
        self.reader = None
        self.lock = threading.Lock()
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            self.port = probe.getsockname()[1]
        self.jar_sha1 = hashlib.sha1(args.server_jar.read_bytes()).hexdigest()
        if self.jar_sha1 != SERVER_SHA1:
            raise AcceptanceFailure("Server JAR does not match pinned official26.3 SHA1")
        with zipfile.ZipFile(args.server_jar) as jar:
            version = json.loads(jar.read("version.json"))
        if version.get("id") != "26.3" or version.get("protocol_version") != 777 or version.get("java_version") != 25:
            raise AcceptanceFailure("Server JAR version/protocol/runtime metadata does not match26.3")
        self.metadata = version
        tested = subprocess.run([str(args.java), "-version"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, timeout=15, creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        if tested.returncode or not re.search(r'version "25(?:\.|\")', tested.stdout):
            raise AcceptanceFailure("This fixture requires a verified Java25 runtime")
        self.java_version = tested.stdout.splitlines()[0]

    @property
    def address(self):
        return "127.0.0.1:" + str(self.port)

    def start(self):
        props = {"server-ip":"127.0.0.1", "server-port":str(self.port), "online-mode":"false",
                 "white-list":"false", "enforce-secure-profile":"false", "level-name":"fixture-world",
                 "level-seed":"42", "level-type":"minecraft\\:flat", "generate-structures":"false",
                 "gamemode":"creative", "force-gamemode":"true", "difficulty":"hard", "view-distance":"4",
                 "simulation-distance":"3", "spawn-protection":"0", "allow-flight":"true", "max-players":"2",
                 "enable-rcon":"false", "enable-query":"false", "enable-jmx-monitoring":"false",
                 "pause-when-empty-seconds":"-1", "motd":"Rusty private Java26.3 QA"}
        (self.directory/"server.properties").write_text("".join(f"{k}={v}\n" for k,v in props.items()))
        (self.directory/"eula.txt").write_text("eula=true\n")
        env = dict(os.environ)
        for name in ["_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS"]:
            env.pop(name, None)
        env.update(HOME=str(self.directory/"home"), USERPROFILE=str(self.directory/"home"),
                   APPDATA=str(self.directory/"appdata"), LOCALAPPDATA=str(self.directory/"localappdata"),
                   TEMP=str(self.directory/"temp"), TMP=str(self.directory/"temp"))
        self.log = (self.directory/"server.log").open("w", encoding="utf-8")
        self.process = subprocess.Popen([str(self.args.java), "-Xms256M", "-Xmx2G", "-Duser.language=en",
            "-Duser.country=US", "-Duser.timezone=UTC", "-Duser.home="+str(self.directory/"home"),
            "-Djava.io.tmpdir="+str(self.directory/"temp"), "-jar", str(self.args.server_jar), "nogui"],
            cwd=self.directory, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, encoding="utf-8", errors="replace", creationflags=getattr(subprocess,"CREATE_NO_WINDOW",0))
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        self.wait(lambda lines: any("Done (" in line for _,line in lines), "Java server ready", timeout=240)

    def _read(self):
        for line in self.process.stdout:
            with self.lock:
                self.serial += 1
                self.lines.append((self.serial, line.rstrip()))
            self.log.write(line); self.log.flush()

    def wait(self, predicate, description, timeout=30, after=0):
        until = time.monotonic()+timeout
        while time.monotonic()<until:
            with self.lock:
                lines = [(serial,line) for serial,line in self.lines if serial>after]
            if predicate(lines): return lines
            if self.process.poll() is not None:
                raise AcceptanceFailure("Owned Java server exited while waiting for "+description)
            time.sleep(.025)
        raise AcceptanceFailure("Timed out waiting for "+description)

    def run(self, command, timeout=30):
        with self.lock: before = self.serial
        self.marker += 1
        marker = "__RUSTY_QA_BARRIER_"+str(self.marker)+"__"
        self.process.stdin.write(command+"\nsay "+marker+"\n"); self.process.stdin.flush()
        lines = self.wait(lambda rows:any(marker in line for _,line in rows), "Java console command barrier", timeout, before)
        result=[line for _,line in lines if marker not in line]
        fatal=["Unknown or incomplete command", "Incorrect argument for command", "That position is not loaded", "Cannot access blocks outside of the world"]
        if any(any(message in line for message in fatal) for line in result):
            raise AcceptanceFailure("Owned Java server rejected fixture command: "+command.split(" ",1)[0])
        return result

    def scalar(self, path, entity=GUEST_NAME):
        lines = self.run(f"data get entity {entity} {path}")
        for line in reversed(lines):
            found = re.search(r"has the following entity data: (.+)$", line)
            if not found: continue
            value = found.group(1).strip()
            number = re.fullmatch(r"([-+]?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?)(?:[fFdDbBsSlL])?", value)
            if number: return float(number.group(1))
            return value[:2048]
        raise AcceptanceFailure("Java entity data path was unavailable: "+path)

    def position(self):
        raw=self.scalar("Pos")
        if not isinstance(raw,str) or not raw.startswith("[") or not raw.endswith("]"):
            raise AcceptanceFailure("Java Pos readback was not a vector")
        parts=raw[1:-1].split(",")
        if len(parts)!=3:raise AcceptanceFailure("Java Pos readback had an unexpected dimension")
        return [float(re.sub(r"[dDfF]$","",value.strip())) for value in parts]

    def item_count(self, item):
        lines = self.run(f"clear {GUEST_NAME} {item} 0")
        for line in reversed(lines):
            found = re.search(r"Found (\d+) matching item", line)
            if found: return int(found.group(1))
            if "No items were found" in line: return 0
        raise AcceptanceFailure("Java inventory count readback did not return a count")

    def boolean_rule(self, name):
        for line in reversed(self.run("gamerule "+name)):
            if name not in line:continue
            found=re.search(r"\b(true|false)\b\s*$",line,re.IGNORECASE)
            if found:return found.group(1).lower()=="true"
        raise AcceptanceFailure("Java boolean gamerule readback was unavailable: "+name)

    def block_matches(self, position, block):
        marker="__RUSTY_QA_AUTHORITATIVE_BLOCK_MATCH__"
        xyz=" ".join(str(value) for value in position)
        rows=self.run(f"execute at {GUEST_NAME} if block {xyz} {block} run say {marker}")
        return any(marker in row for row in rows)

    def has_item(self, slot, item, count=None):
        """Query actual inventory NBT without consuming an item or trusting prediction."""
        # Java26.3 moved player equipment out of the 36-slot Inventory list.
        equipment={-106:"offhand",100:"feet",101:"legs",102:"chest",103:"head"}
        fields=f'id:"{item}"'
        if count is not None: fields+=f",count:{count}"
        path=(f"equipment.{equipment[slot]}{{{fields}}}" if slot in equipment else
              f"Inventory[{{Slot:{slot}b,{fields}}}]")
        marker="__RUSTY_QA_INVENTORY_MATCH__"
        rows=self.run(f"execute if data entity {GUEST_NAME} {path} run say {marker}")
        return any(marker in row for row in rows)

    def inventory_receipt(self, name):
        rows=self.run(f"data get entity {GUEST_NAME} Inventory")
        raw=next((line.split("has the following entity data: ",1)[1] for line in rows if "has the following entity data: " in line),None)
        if raw is None: raise AcceptanceFailure("Java Inventory NBT readback was unavailable")
        (self.directory/("inventory-"+name+".txt")).write_text(raw,encoding="utf-8")
        return {"inventory_nbt_sha256":hashlib.sha256(raw.encode()).hexdigest(),
                "selected_slot_zero_based":self.scalar("SelectedItemSlot"),
                "player_health":self.scalar("Health"),"player_food":self.scalar("foodLevel")}

    def stop(self):
        if self.process is not None and self.process.poll() is None:
            try:
                self.process.stdin.write("stop\n"); self.process.stdin.flush(); self.process.wait(timeout=30)
            except (OSError, ValueError, subprocess.TimeoutExpired):
                self.process.terminate()
                try: self.process.wait(timeout=10)
                except subprocess.TimeoutExpired: self.process.kill(); self.process.wait(timeout=10)
        if self.reader is not None: self.reader.join(timeout=5)
        if self.log is not None: self.log.close()


class JavaPeer(Peer, InventorySmoke, AwarenessSmoke, BuilderSmoke):
    def __init__(self, args, server):
        self.role, self.deadline, self.stopping = "guest", None, False
        FeatureSmoke.__init__(self,args)
        self.java = server
        self.failed_cases = []
        self.report.update(suite="vanilla-java-client-peer", authoritative_server="OfficialJava26.3",
            server_jar_sha1=server.jar_sha1, java_version=server.java_version,
            account_loaded=False, server_readbacks=[],
            not_exercised=["Online authenticated servers, other server versions/plugins, anticheat behavior and long sessions.",
                           "This suite does not compare a fired projectile against its read-only preview or validate randomized spread; screenshots need separate visual review."])
        self.report["claims"]="Native client behavior and independently queried officialJava server state; screenshot appearance requires review."
        for key in ["pixel_contract","fixture_correction"]:
            self.report.pop(key,None)
        self.authoritative = {}
        self._tool_confirmed = False
        self._hunger_started = False
        self._trajectory_inventory_snapshot = None

    def launch(self, address=None):
        if self.process is not None: return self.ready()
        return Peer.launch(self,self.java.address)

    def server(self, text):
        mainhand_prefix="item replace entity @s weapon.mainhand with "
        expected_item=None
        if text.startswith(mainhand_prefix):
            expected_item=text[len(mainhand_prefix):].split("[",1)[0].split(" ",1)[0]
            native_slot=self.command("inventory")["selected"]-1
            self.wait_for(lambda:self.java.scalar("SelectedItemSlot"),lambda slot:slot==native_slot,
                          "Java confirms the selected slot before a main-hand fixture replacement",timeout=3)
        if self.phase == "AutoTotem" and text == "item replace entity @s weapon.offhand with minecraft:air":
            equipment=self.java.scalar("equipment")
            self.authoritative["protected_equipment_nbt_sha256"]=hashlib.sha256(str(equipment).encode()).hexdigest()
            self.require(self.java.has_item(-106,"minecraft:shield") and self.java.has_item(4,"minecraft:totem_of_undying",1),
                         "Java server did not retain protected shield and source totem before deliberate offhand clearing")
            self.authoritative["protected_offhand_shield_confirmed"]=True
        if self.phase == "AutoRefill" and text == "item replace entity @s hotbar.0 with minecraft:stone 15":
            self.require(self.java.has_item(0,"minecraft:stone",16) and
                         not self.java.has_item(10,"minecraft:stone") and self.java.has_item(9,"minecraft:stone",12),
                         "Java server did not confirm the exact plain-stone refill and preserved named source")
            self.authoritative["intermediate_refill_confirmed"]={"hotbar_count":16,"source_empty":True,"named_source_count":12}
        result = self.java.run(f"execute as {GUEST_NAME} at @s run {text}")
        if self.phase in {"AutoRefillTool","AutoRefillControllerTool"} and text in {"gamerule block_drops false","gamerule block_drops true"}:
            expected=text.endswith("true")
            self.require(self.java.boolean_rule("block_drops")==expected,
                         "Java controlled tool fixture did not confirm the requested block-drop rule")
            self.authoritative["block_drops_restored_true" if expected else "block_drops_disabled_false"]=True
        if expected_item is not None:
            received=self.wait_for(lambda:self.command("inventory"),
                lambda inv:inv["selected"]-1==native_slot and (inv.get("held") or {}).get("item")==expected_item,
                "received authoritative main-hand fixture item",timeout=5)
            self.report.setdefault("mainhand_fixture_barriers",[]).append({"item":expected_item,
                "server_selected_slot_zero_based":native_slot,"received_inventory_sha256":canonical_digest(received)})
            if expected_item=="minecraft:ender_pearl":self._trajectory_inventory_snapshot="before"
        if self.phase == "TriggerBot" and text.startswith("summon minecraft:cow"):
            self.authoritative["cow_health_before"] = self.java.scalar("Health", "@e[type=minecraft:cow,limit=1]")
        if self.phase == "AutoEat" and "effect give @s minecraft:hunger" in text:
            self._hunger_started = True
        if self.phase == "AutoEat" and text == "effect clear @s" and self._hunger_started:
            self.authoritative["food_before"] = self.java.scalar("foodLevel")
        return {"ok":True}

    def command(self,line,timeout=15):
        if line.split(" ",1)[0] in {"command","chat","tp","fill","give","time"}:
            raise AcceptanceFailure("Java guest attempted an authoritative client shortcut")
        try:
            endpoint=json.loads(self.endpoint.read_text(encoding="utf-8"))
            with socket.create_connection(("127.0.0.1",int(endpoint["port"])),timeout=timeout) as connection:
                connection.settimeout(timeout)
                with connection.makefile("rwb") as stream:
                    stream.write(("auth "+endpoint["token"]+"\n").encode());stream.flush()
                    if not json.loads(stream.readline(1024*1024)).get("ok"):
                        raise AcceptanceFailure("Private control authentication failed")
                    stream.write((line+"\n").encode());stream.flush()
                    value=json.loads(stream.readline(16*1024*1024))
            if not isinstance(value,dict):raise AcceptanceFailure("Native reply was not an object")
            if "error" in value:
                # Local diagnostic receipt only. Tokens/authentication messages never enter it.
                with (self.output/"control-errors-private.jsonl").open("a",encoding="utf-8") as record:
                    record.write(json.dumps({"phase":self.phase,"command":line.split(" ",1)[0],"error":str(value["error"])[:2048]})+"\n")
                raise AcceptanceFailure("Game rejected the "+line.split(" ",1)[0]+" command; see private control error receipt")
        except (OSError,ValueError,KeyError,TypeError) as exc:
            raise AcceptanceFailure("Private native control failed: "+type(exc).__name__) from None
        if self.phase == "AutoTool" and line == "inventory" and value.get("selected") == 2 and not self._tool_confirmed:
            try: enabled = self.config()["assistance"]["auto_tool"]
            except (OSError, KeyError, AcceptanceFailure): enabled = False
            if enabled:
                until=time.monotonic()+2
                actual=self.java.scalar("SelectedItemSlot")
                while actual!=1 and time.monotonic()<until:
                    time.sleep(.05); actual=self.java.scalar("SelectedItemSlot")
                self.require(actual==1,"Client AutoTool prediction did not reach Java server selected slot")
                self.authoritative["automatic_tool_slot_zero_based"] = actual
                self._tool_confirmed=True
        if line=="inventory" and self._trajectory_inventory_snapshot is not None:
            when=self._trajectory_inventory_snapshot
            self.report["trajectory_inventory_"+when]=value
            self._trajectory_inventory_snapshot="after" if when=="before" else None
        return value

    def set_module(self,module,path,enabled=True):
        if self.phase == "SafeWalk" and module=="safe_walk" and not enabled:
            try: before=self.config()["assistance"]["safe_walk"]
            except (AcceptanceFailure, KeyError): before=False
            if before:
                position=self.java.position()
                self.require(position[1]>199.9,"Java server observed falling while SafeWalk was enabled")
                self.authoritative["safe_walk_server_position"]=position
        return FeatureSmoke.set_module(self,module,path,enabled)

    def obs(self): return self.state()["awareness"]
    def enable(self,module,on=True): return self.set_module(module,["awareness",module],on)

    def photo(self,name,*args):
        if name in ["08-bow-block-trajectory","09-pearl-block-trajectory"]:
            state=self.wait_for(self.obs,lambda o:o.get("prediction",{}).get("segments",0)>0,"Java-supplied held item trajectory geometry")
            self.report.setdefault("java_trajectory_observations",[]).append(state["prediction"])
        return FeatureSmoke.photo(self,name,*args)

    def finish(self):
        self.stopping=True
        Smoke.finish(self)
        self.endpoint.unlink(missing_ok=True)
        (self.output/"gui-acceptance.json").replace(self.output/"java-peer-receipt.json")


class JavaSuite:
    def __init__(self,args):
        self.args=args
        self.output=args.output.resolve();self.output.mkdir(parents=True,exist_ok=False)
        self.server=None;self.peer=None;self.phase="setup"
        self.report={"schema":1,"suite":"official-java26.3-compatibility","script_sha256":digest(Path(__file__)),
            "case_selection":("repeated_single_refill_only" if args.only_refill_single else
                              "controller_refill_only" if args.only_controller_refill else
                              "repeated_tool_refill_only" if args.only_refill_tool else
                              "equipment_only" if args.only_equipment else
                              "full_except_separately_recorded_refill_edge" if args.skip_refill_single else "full"),
            "fixture_sources":{name:digest(Path(__file__).parent/name) for name in [
                "game_assistance_smoke.py","game_automation_smoke.py","game_inventory_smoke.py",
                "game_builder_smoke.py","game_awareness_smoke.py"]},
            "checks":[],"passed":False,"offline_loopback_only":True,"real_account_used":False,
            "limitations":["One officialJava26.3server, offline mode and fresh controlled worlds; no claim of full server/plugin/anticheat compatibility.",
                           "Authoritative Java NBT/slot/item-count readbacks are distinguished from native client prediction."]}

    def prepare(self,dimension="minecraft:overworld",ores=False):
        prefix=f"execute in {dimension} run "
        self.server.run(prefix+"forceload add 0 0 31 31")
        until=time.monotonic()+90
        while time.monotonic()<until:
            rows=self.server.run(prefix+"execute if loaded 0 94 0 if loaded 31 94 31 run say __AREA_READY__")
            if any("__AREA_READY__" in line for line in rows):break
            time.sleep(.2)
        else:raise AcceptanceFailure("Java fixture chunks did not load")
        for cmd in ["fill 0 95 0 31 105 31 air","fill 0 94 0 31 94 31 stone"]:
            self.server.run(prefix+cmd)
        if ores:
            for cmd in ["fill 4 95 8 12 100 12 stone","fill 14 95 8 17 100 12 stone",
                        "setblock 8 97 10 diamond_ore","setblock 15 97 10 diamond_ore","setblock 16 97 10 diamond_ore"]:
                self.server.run(prefix+cmd)

    def teleport(self,dimension):
        self.server.run(f"execute in {dimension} run tp {GUEST_NAME} 8.5 95 3.5 0 0")
        self.peer.wait_for(lambda:self.peer.command("status"),lambda s:s["dimension"]==dimension and s.get("player") and abs(s["player"]["position"][0]-8.5)<.05 and abs(s["player"]["position"][1]-95)<.05 and abs(s["player"]["position"][2]-3.5)<.05,"Java authoritative teleport",timeout=90)
        self.peer.wait_render_ready()

    def check(self,name,**details):
        self.report["checks"].append({"name":name,"passed":True,**details});print("PASS "+name,flush=True)

    def refill_single(self, variant="single"):
        names={"single":"AutoRefillSingle","resume":"AutoRefillResume","reenable":"AutoRefillReenable",
               "held_resume":"AutoRefillHeldResume","controller":"AutoRefillController",
               "controller_held_resume":"AutoRefillControllerHeldResume"}
        self.phase=self.peer.phase=names[variant]
        self.peer.off();self.peer.authoritative={}
        try:
            {"single":self.peer.refill_single,"resume":self.peer.refill_resume,
             "reenable":self.peer.refill_reenable,"held_resume":self.peer.refill_held_resume,
             "controller":self.peer.refill_controller,
             "controller_held_resume":self.peer.refill_controller_held_resume}[variant]()
            self.peer.require(self.server.has_item(0,"minecraft:stone",1) and
                              not self.server.has_item(9,"minecraft:stone") and
                              not self.server.has_item(10,"minecraft:stone"),
                              "Java server did not confirm the second single-stack refill and both emptied reserves")
            marker="__RUSTY_JAVA_SINGLE_REFILL_CONFIRMED__"
            rows=self.server.run(f"execute at {GUEST_NAME} if block 0 200 1 minecraft:stone if block -1 200 0 minecraft:stone run say {marker}")
            self.peer.require(any(marker in row for row in rows),"Java server did not confirm both ordinary single-item placements")
            position=self.server.position()
            self.peer.require(all(abs(position[i]-[.5,200,.5][i])<.01 for i in range(3)),
                              "Java server body moved during the same-context repeated refill check")
            descriptions={"single":"without menu or toggle","resume":"after menu suspension and a fresh user press",
                          "reenable":"after its own gameplay binding toggles while Armor remains enabled",
                          "held_resume":"after held input is released beneath the menu and a fresh user press",
                          "controller":"after fresh ordinary controller USE presses",
                          "controller_held_resume":"after held controller USE is released beneath the menu and a fresh controller press"}
            self.check("Java authoritative repeated single-item refill "+descriptions[variant],
                       placed_blocks=2,hotbar_count=1,reserves_empty=True,
                       unchanged_server_position=position,
                       server_readbacks=self.server.inventory_receipt(names[variant]))
        except AcceptanceFailure:
            self.report.setdefault("refill_failures",{})[variant]={"native_inventory":self.peer.command("inventory"),
                "native_inventory_assist":self.peer.command("rusty-status").get("inventory_assist"),
                "server_readbacks":self.server.inventory_receipt(names[variant]+"Failure"),
                "server_position":self.server.position(),
                "first_block_stone":self.server.block_matches([0,200,1],"minecraft:stone"),
                "second_block_stone":self.server.block_matches([-1,200,0],"minecraft:stone")}
            raise
        finally:self.peer.command("release")

    def refill_tool(self, controller=False):
        name="AutoRefillControllerTool" if controller else "AutoRefillTool"
        self.phase=self.peer.phase=name
        self.peer.off();self.peer.authoritative={}
        try:
            (self.peer.refill_controller_tool if controller else self.peer.refill_tool)()
            self.peer.require(self.peer.authoritative.get("block_drops_disabled_false") is True and
                              self.peer.authoritative.get("block_drops_restored_true") is True,
                              "Java tool fixture did not confirm drop suppression and restoration")
            self.peer.require(self.server.has_item(0,"minecraft:wooden_pickaxe",1) and
                              not self.server.has_item(9,"minecraft:wooden_pickaxe") and
                              not self.server.has_item(10,"minecraft:wooden_pickaxe"),
                              "Java server did not confirm both broken-tool replacements and emptied reserves")
            damage=self.server.scalar('Inventory[{Slot:0b}].components."minecraft:damage"')
            self.peer.require(damage==58,"Java replacement tool did not retain the exact source durability")
            marker="__RUSTY_JAVA_TOOL_REFILL_CONFIRMED__"
            rows=self.server.run(f"execute at {GUEST_NAME} if block 0 200 2 minecraft:air if block -2 200 0 minecraft:air run say {marker}")
            self.peer.require(any(marker in row for row in rows),"Java server did not confirm both ordinary mined blocks")
            position=self.server.position()
            self.peer.require(all(abs(position[i]-[.5,200,.5][i])<.01 for i in range(3)),
                              "Java server body moved during the same-context repeated tool refill check")
            self.check("Java authoritative repeated broken-tool refill after fresh ordinary "+("controller attacks" if controller else "attacks"),
                       mined_blocks=2,hotbar_tool_count=1,remaining_damage=damage,reserves_empty=True,
                       unchanged_server_position=position,
                       server_readbacks={**self.peer.authoritative,**self.server.inventory_receipt(name)})
        except AcceptanceFailure:
            self.report["controller_refill_tool_failure" if controller else "refill_tool_failure"]={"native_inventory":self.peer.command("inventory"),
                "native_inventory_assist":self.peer.command("rusty-status").get("inventory_assist"),
                "server_readbacks":self.server.inventory_receipt(name+"Failure")}
            raise
        finally:self.peer.command("release")

    def core_remote(self):
        peer=self.peer
        self.phase="Java join and received Xray updates"
        self.prepare(ores=True)
        self.server.run(f"gamemode survival {GUEST_NAME}")
        self.teleport("minecraft:overworld")
        peer.wait_for(lambda:peer.command("block 8 97 10"),lambda b:b["loaded"] and b["block"]=="minecraft:diamond_ore","Java ore arrives")
        source=canonical_digest(peer.command("blocks 4 95 8 17 100 12"))
        peer.key("key.keyboard.x");scan=peer.scan_ready(3)
        peer.require(peer.state()["xray"],"Java guest Xray did not activate")
        peer.require(canonical_digest(peer.command("blocks 4 95 8 17 100 12"))==source,"Xray mutated received Java source")
        peer.photo("java-01-xray-received")
        self.server.run("setblock 8 97 10 stone")
        peer.wait_for(lambda:peer.command("block 8 97 10"),lambda b:b["loaded"] and b["block"]=="minecraft:stone","Java ore removal received")
        peer.wait_for(peer.state,lambda s:s["pending"]==0 and s["targets"]==scan["targets"]-1,"Java block update refreshes scan",timeout=90)
        self.server.run("setblock 8 97 10 diamond_ore")
        peer.wait_for(lambda:peer.command("block 8 97 10"),lambda b:b["loaded"] and b["block"]=="minecraft:diamond_ore","Java ore restoration received")
        peer.key("key.keyboard.x")
        peer.wait_for(peer.state,lambda s:not s["xray"],"Java Xray restores normal rendering")
        peer.require(canonical_digest(peer.command("blocks 4 95 8 17 100 12"))==source,"Java original source was not restored")
        peer.require(peer.command("status")["server"]["decode_errors"]==0,"Java join/play packet decode errors")
        self.check("OfficialJava26.3join and authoritative Xray source/update/restoration",targets=scan["targets"],source_sha256=source,decode_errors=0)
        peer.typed_command("rusty waypoint add QA_Java_Origin")
        peer.wait_waypoint("QA_Java_Origin")
        self.phase="Java dimension and reconnect"
        self.prepare("minecraft:the_nether")
        peer.key("key.keyboard.x");peer.wait_for(peer.state,lambda s:s["xray"],"Xray before Java dimension change")
        self.teleport("minecraft:the_nether")
        peer.wait_for(peer.state,lambda s:not s["xray"],"Java dimension change resets activation")
        peer.typed_command("rusty waypoint add QA_Java_Nether")
        data=peer.wait_waypoint("QA_Java_Nether")
        origin=next(s for s in data["scopes"] if any(w["name"]=="QA_Java_Origin" for w in s["waypoints"]))
        nether=next(s for s in data["scopes"] if any(w["name"]=="QA_Java_Nether" for w in s["waypoints"]))
        peer.require(origin["scope"]["dimension"]=="minecraft:overworld" and nether["scope"]["dimension"]=="minecraft:the_nether" and origin["scope"]["identity"]==nether["scope"]["identity"],"Java dimension waypoint isolation failed")
        self.teleport("minecraft:overworld")
        peer.command("disconnect",30)
        peer.wait_for(lambda:peer.command("status"),lambda s:s["screen"]=="title" and s["server"] is None,"Java guest disconnect")
        peer.command("connect "+self.server.address+" "+GUEST_NAME,30);peer.ready();self.teleport("minecraft:overworld")
        peer.wait_for(lambda:peer.command("block 8 97 10"),lambda b:b["loaded"] and b["block"]=="minecraft:diamond_ore","Java reconnect receives original ore")
        peer.require(peer.command("status")["server"]["decode_errors"]==0,"Java reconnect packet decode errors")
        self.check("OfficialJava dimension lifecycle, scoped waypoints and same-endpoint reconnect",decode_errors=0)

    def run(self):
        self.server=JavaServer(self.args,self.output)
        print("Verified official26.3 JAR and Java25; starting private loopback server",flush=True)
        self.server.start()
        print("Private officialJava26.3 server ready",flush=True)
        for cmd in ["gamerule advance_time false","gamerule advance_weather false","gamerule spawn_mobs false","time set noon","weather clear"]:
            self.server.run(cmd)
        options=argparse.Namespace(**vars(self.args));options.output=self.output/"client"
        self.peer=JavaPeer(options,self.server);self.peer.launch()
        print("Native client joined owned Java server",flush=True)
        self.report.update(binary_sha256=self.peer.report["binary_sha256"],server_jar_sha1=self.server.jar_sha1,
            java_version=self.server.java_version,server_protocol=self.server.metadata["protocol_version"])
        self.peer.key("key.keyboard.right.shift")
        self.peer.wait_for(self.peer.ui,lambda s:s["open"] and bool(s["hits"]),"initial assistance profile")
        self.peer.search("safe walk");self.peer.click('Select("safe_walk")');self.peer.key("key.keyboard.escape")
        if self.args.only_refill_single:
            self.refill_single();self.report["passed"]=True;return
        if self.args.only_controller_refill:
            self.refill_single("controller");self.report["passed"]=True;return
        if self.args.only_refill_tool:
            self.refill_tool();self.report["passed"]=True;return
        if not self.args.only_equipment:self.core_remote()
        original_cases=[("SafeWalk",self.peer.walk),("AutoJump",self.peer.jump),("AutoTool",self.peer.tool),("TriggerBot",self.peer.trigger),("AutoEat",self.peer.eat)]
        for name,case in ([] if self.args.only_equipment else original_cases):
            self.phase=self.peer.phase=name
            try:
                self.peer.off();self.peer.authoritative={};case()
                if name=="AutoJump":
                    actual=self.server.position();self.peer.require(actual[2]>2.8,"Java server did not confirm jump traversal");self.peer.authoritative["auto_jump_server_position"]=actual
                elif name=="AutoTool":
                    actual=self.server.scalar("SelectedItemSlot");self.peer.require(self.peer._tool_confirmed and actual==3,"Java server did not confirm automatic and manual hotbar slots");self.peer.authoritative["manual_tool_slot_zero_based"]=actual
                elif name=="TriggerBot":
                    actual=self.server.scalar("Health","@e[type=minecraft:cow,limit=1]");self.peer.require(actual<self.peer.authoritative["cow_health_before"],"Java authoritative cow health did not decrease");self.peer.authoritative["cow_health_after"]=actual
                elif name=="AutoEat":
                    actual=self.server.scalar("foodLevel");bread=self.server.item_count("minecraft:bread");poison=self.server.item_count("minecraft:poisonous_potato")
                    self.peer.require(actual>self.peer.authoritative["food_before"] and bread<12 and poison==12,"Java server did not confirm food increase and safe bread consumption")
                    self.peer.authoritative.update(food_after=actual,bread_after=bread,poisonous_potato_after=poison)
                self.peer.authoritative.update(self.server.inventory_receipt(name))
                self.check("Java authoritative "+name,server_readbacks=dict(self.peer.authoritative))
            except (AcceptanceFailure,KeyError,ValueError,TypeError) as exc:
                self.report["checks"].append({"name":"Java "+name,"passed":False,"reason":str(exc)})
                print("FAIL Java "+name+": "+str(exc),flush=True)
            finally:self.peer.command("release")
        item_cases=[("AutoTotem",self.peer.totem),("AutoFish",self.peer.fishing),
                    ("ArmorAssist",self.peer.armor),("AutoRefill",self.peer.refill)]
        if self.args.only_equipment:item_cases=[case for case in item_cases if case[0] in {"AutoTotem","ArmorAssist"}]
        for name,case in item_cases:
            self.phase=self.peer.phase=name
            try:
                self.peer.off();self.peer.authoritative={};case()
                if name=="AutoTotem":
                    self.peer.require(self.peer.authoritative.get("protected_offhand_shield_confirmed") and
                        self.server.has_item(-106,"minecraft:totem_of_undying",1) and
                        not self.server.has_item(4,"minecraft:totem_of_undying") and
                        self.server.item_count("minecraft:totem_of_undying")==1 and
                        self.server.scalar("SelectedItemSlot")==0,
                        "Java server did not confirm one protected-offhand to empty-offhand swap and selected-slot restore")
                    self.peer.authoritative.update(offhand_totem_count=1,source_totem_slot_empty=True,total_totems=1)
                elif name=="AutoFish":
                    native=self.peer.command("inventory")
                    loot={item:self.server.item_count(item) for item,count in native["counts"].items()
                          if item not in {"minecraft:fishing_rod","minecraft:stick","minecraft:air"} and count>0}
                    self.peer.require(bool(loot) and all(count>0 for count in loot.values()),
                                      "Received fishing loot was not independently present in Java server inventory")
                    self.peer.require(self.server.scalar("SelectedItemSlot")==1,
                                      "Java server did not retain the manual post-fishing slot selection")
                    self.peer.authoritative.update(caught_items=loot,manual_slot_retained=True,
                                                  natural_bite_observed=True)
                elif name=="ArmorAssist":
                    equipment=self.server.scalar("equipment")
                    self.peer.authoritative["equipped_nbt_sha256"]=hashlib.sha256(str(equipment).encode()).hexdigest()
                    expected=[(103,"minecraft:iron_helmet"),(102,"minecraft:iron_chestplate"),
                              (100,"minecraft:leather_boots"),(9,"minecraft:diamond_helmet"),
                              (12,"minecraft:diamond_boots")]
                    self.peer.require(all(self.server.has_item(slot,item,1) for slot,item in expected) and
                        not self.server.has_item(10,"minecraft:iron_helmet") and
                        not self.server.has_item(11,"minecraft:iron_chestplate") and
                        not self.server.has_item(13,"minecraft:iron_helmet"),
                        "Java server did not confirm empty-part armor moves and protected equipment/source slots")
                    curse=self.server.scalar('Inventory[{Slot:9b}].components."minecraft:enchantments"')
                    self.peer.require("binding_curse" in str(curse),"Java protected helmet did not retain its binding curse")
                    self.peer.authoritative.update(equipped_head="minecraft:iron_helmet",equipped_chest="minecraft:iron_chestplate",
                                                  protected_feet="minecraft:leather_boots",binding_helmet_preserved=True,
                                                  diamond_boots_source_preserved=True,repeated_head_fill_confirmed=True)
                else:
                    self.peer.require(self.peer.authoritative.get("intermediate_refill_confirmed") and
                        not self.server.has_item(0,"minecraft:stone") and not self.server.has_item(10,"minecraft:stone") and
                        self.server.has_item(9,"minecraft:stone",12),
                        "Java server did not preserve the component-mismatching source after the target was emptied again")
                    named=self.server.scalar('Inventory[{Slot:9b}].components."minecraft:custom_name"')
                    self.peer.require("Named" in str(named),"Java mismatch source lost its distinguishing custom name")
                    marker="__RUSTY_JAVA_REFILL_PLACED__"
                    rows=self.server.run(f"execute at {GUEST_NAME} if block 0 200 1 minecraft:stone run say {marker}")
                    self.peer.require(any(marker in row for row in rows),"Java server did not confirm the user placement exhausting the original stack")
                    self.peer.authoritative.update(named_source_retained=True,final_target_empty=True,ordinary_placed_block="minecraft:stone")
                self.peer.authoritative.update(self.server.inventory_receipt(name))
                self.check("Java authoritative "+name,server_readbacks=dict(self.peer.authoritative))
            except (AcceptanceFailure,KeyError,ValueError,TypeError) as exc:
                self.report["checks"].append({"name":"Java "+name,"passed":False,"reason":str(exc)})
                print("FAIL Java "+name+": "+str(exc),flush=True)
            finally:self.peer.command("release")
        if self.args.only_equipment:
            self.peer.off();self.report["passed"]=all(c["passed"] for c in self.report["checks"])
            if not self.report["passed"]:raise AcceptanceFailure("One or more Java equipment cases failed")
            return
        refill_cases=[] if self.args.skip_refill_single else [("AutoRefillSingle",self.refill_single)]
        refill_cases.extend([("AutoRefillResume",lambda:self.refill_single("resume")),
                             ("AutoRefillReenable",lambda:self.refill_single("reenable")),
                             ("AutoRefillHeldResume",lambda:self.refill_single("held_resume")),
                             ("AutoRefillController",lambda:self.refill_single("controller")),
                             ("AutoRefillControllerHeldResume",lambda:self.refill_single("controller_held_resume"))])
        refill_cases.append(("AutoRefillTool",self.refill_tool))
        refill_cases.append(("AutoRefillControllerTool",lambda:self.refill_tool(True)))
        for name,case in refill_cases:
            try:case()
            except (AcceptanceFailure,KeyError,ValueError,TypeError) as exc:
                self.report["checks"].append({"name":"Java "+name,"passed":False,"reason":str(exc)})
                print("FAIL Java "+name+": "+str(exc),flush=True)
        self.peer.off()
        self.phase=self.peer.phase="Java ordinary waypoint route"
        for command in ["fill 0 120 0 24 125 8 air", "fill 0 119 0 24 119 8 stone", "gamemode survival @s"]:
            self.peer.server(command)
        for x in [8,20]:
            self.peer.wait_for(lambda x=x:self.peer.command(f"block {x} 119 3"),lambda block:block["loaded"] and block["block"]=="minecraft:stone","Java route support received")
        self.peer.server("tp @s 20.5 120 3.5 0 0")
        self.peer.wait_for(self.peer.player,lambda p:abs(p["position"][0]-20.5)<.05 and abs(p["position"][1]-120)<.05 and p["on_ground"],"Java route marker destination")
        self.peer.typed_command("rusty waypoint add QA_Java_Walk")
        self.peer.wait_waypoint("QA_Java_Walk")
        self.peer.server("tp @s 8.5 120 3.5 0 0")
        self.peer.wait_for(self.peer.player,lambda p:abs(p["position"][0]-8.5)<.05 and abs(p["position"][2]-3.5)<.05 and p["on_ground"],"Java route origin")
        self.peer.typed_command("rusty waypoint walk")
        self.peer.wait_for(lambda:self.peer.command("rusty-walk-status"),lambda state:not state["active"] and state["arrived"],"Java walking task arrival",timeout=20)
        actual=self.server.position()
        self.peer.require(abs(actual[0]-20.5)<.8 and abs(actual[2]-3.5)<.8,"Client route arrival was not confirmed by Java server position")
        self.check("WaypointWalk reaches selected destination on officialJava server",server_position=actual)
        self.phase=self.peer.phase="Java ordinary builder placement"
        BuilderSmoke.exercise(self.peer)
        marker="__RUSTY_JAVA_BUILDER_CONFIRMED__"
        rows=self.server.run(f"execute at {GUEST_NAME} if block 0 200 1 minecraft:cobblestone run say {marker}")
        self.peer.require(any(marker in row for row in rows) and self.server.has_item(0,"minecraft:cobblestone",63),
                          "Java server did not confirm the single placed block and exact one-item consumption")
        self.check("Java authoritative single ordinary builder placement and bounded passive preview",
                   server_block="minecraft:cobblestone",server_item_count=63,
                   server_readbacks=self.server.inventory_receipt("BuildPlanner"))
        self.phase=self.peer.phase="Java received awareness"
        self.peer.off()
        config=self.peer.config()
        activation_keys=["safe_walk","auto_jump","auto_tool","trigger_bot","auto_eat",
                         "auto_fish","auto_totem","armor_assist","refill"]
        activation={key:config["assistance"][key] for key in activation_keys}
        native=self.peer.state()
        self.report["awareness_preconditions"]={"assistance_activation":activation,
            "build_enabled":config["build"]["enabled"],"freecam_active":native["freecam"],
            "waypoint_walk_active":native["walking"]["active"],"menu_open":self.peer.ui()["open"]}
        self.peer.require(all(value is False for value in activation.values()) and
            config["build"]["enabled"] is False and native["freecam"] is False and
            native["walking"]["active"] is False and not self.peer.ui()["open"],
            "Read-only awareness fixture still had another action feature or menu active")
        self.peer.server("give @s minecraft:arrow 64")
        AwarenessSmoke.exercise(self.peer)
        self.check("Java-supplied storage, light, trajectories, tracers and world HUD",trajectory_observations=self.peer.report.get("java_trajectory_observations",[]))
        self.peer.require(self.peer.command("status")["server"]["decode_errors"]==0,"Java final packet decode errors")
        self.report["passed"]=all(c["passed"] for c in self.report["checks"])
        if not self.report["passed"]:
            self.phase="Java suite aggregate"
            raise AcceptanceFailure("One or more Java authoritative assistance cases failed")

    def finish(self):
        errors=[]
        try:
            if self.peer is not None:
                self.peer.report["passed"]=self.report["passed"]
                self.peer.finish()
        except (OSError,ValueError,subprocess.SubprocessError) as exc:errors.append(type(exc).__name__)
        finally:
            try:
                if self.server is not None:self.server.stop()
            except (OSError,ValueError,subprocess.SubprocessError) as exc:errors.append(type(exc).__name__)
        self.report["cleanup_errors"]=errors
        if errors:self.report["passed"]=False
        self.report["owned_processes_stopped"]=(self.peer is None or self.peer.process is None or self.peer.process.poll() is not None) and (self.server is None or self.server.process is None or self.server.process.poll() is not None)
        if not self.report["owned_processes_stopped"]:self.report["passed"]=False
        (self.output/"java-server-acceptance.json").write_text(json.dumps(self.report,indent=2)+"\n")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ["binary","resources","server-jar","java","output"]:parser.add_argument("--"+name,required=True,type=Path)
    parser.add_argument("--width",type=int,default=960);parser.add_argument("--height",type=int,default=540)
    parser.add_argument("--startup-timeout",type=float,default=240)
    edge=parser.add_mutually_exclusive_group()
    edge.add_argument("--only-refill-single",action="store_true",help="Run the discriminating repeated single-stack refill case first")
    edge.add_argument("--only-controller-refill",action="store_true",help="Run the stationary repeated ordinary controller USE refill case")
    edge.add_argument("--only-refill-tool",action="store_true",help="Run the repeated last-durability tool replacement case")
    edge.add_argument("--skip-refill-single",action="store_true",help="Collect the other cases after an independently recorded refill failure")
    edge.add_argument("--only-equipment",action="store_true",help="Verify the actual26.3 equipment NBT paths and ordinary totem/armor actions")
    args=parser.parse_args()
    for name in ["binary","resources","server_jar","java"]:setattr(args,name,getattr(args,name).resolve(strict=True))
    suite=None
    try:
        suite=JavaSuite(args);suite.run();return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError,KeyboardInterrupt) as exc:
        if suite:
            suite.report["passed"]=False;suite.report["checks"].append({"name":suite.phase,"passed":False,"reason":str(exc)})
        print("FAIL "+str(exc),file=sys.stderr,flush=True);return 1
    finally:
        if suite:suite.finish()


if __name__=="__main__":raise SystemExit(main())
