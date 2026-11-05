"""Controlled disposable-world native build comparison. Never touches user worlds."""
import argparse,hashlib,json,pathlib,sys,time,psutil,os,subprocess,socket
class Probe:
 def command(self,name,text,timeout=40):
  endpoint=json.loads((self.LIVE/name/"control.json").read_text())
  with socket.create_connection(("127.0.0.1",int(endpoint["port"])),timeout=timeout) as sock:
   sock.settimeout(timeout)
   with sock.makefile("rwb") as stream:
    stream.write(("auth "+endpoint["token"]+"\n").encode());stream.flush()
    if not json.loads(stream.readline(1024*1024)).get("ok"):raise RuntimeError("Private control authentication failed")
    stream.write((text+"\n").encode());stream.flush();reply=json.loads(stream.readline(16*1024*1024))
  if not isinstance(reply,dict) or "error" in reply:raise RuntimeError("Owned game rejected "+text.split(" ",1)[0])
  return reply
 def settled(self,name,timeout=90):
  until=time.monotonic()+timeout;stable=0
  while time.monotonic()<until:
   value=self.command(name,"status",5)
   ready=value.get("player") and value.get("chunks",0)>=9 and value.get("pending_chunks",1)==0 and value.get("pending_uploads",1)==0 and value.get("streaming",{}).get("meshing",1)==0
   stable=stable+1 if ready else 0
   if stable>=4:return value
   time.sleep(.3)
  raise RuntimeError("Owned vanilla scene did not settle")
 def shot(self,name,label):
  self.command(name,"screenshot "+str((self.LIVE/(label+".png")).resolve()))
probe=Probe()
p=argparse.ArgumentParser(description="Fresh offline vanilla-world performance observation; no installed data is used.")
p.add_argument("--binary",required=True,type=pathlib.Path)
p.add_argument("--resources",required=True,type=pathlib.Path)
p.add_argument("--output",required=True,type=pathlib.Path)
p.add_argument("--samples",type=int,default=3)
p.add_argument("--disable-mod",action="store_true")
p.add_argument("--require-scan-cache",action="store_true",help="Require native stationary Xray cache-reuse telemetry in the candidate")
a=p.parse_args()
if not 1<=a.samples<=10:p.error("samples must be1..10")
a.label=a.output.name
probe.BIN=a.binary.resolve(strict=True);probe.RES=a.resources.resolve(strict=True)
probe.LIVE=a.output.resolve();probe.LIVE.mkdir(parents=True,exist_ok=False)
name="host";home=probe.LIVE/name;home.mkdir(exist_ok=True)
other=["appleskin","armor_status","biomecompass","chatheads","coords_hud","distant_horizons","eatinganimation","enchantmenthints","fps_overlay","itemnew","rightharvest","serverify"]
(home/"horizon-mods.txt").write_text("mod.rusty:"+str(not a.disable_mod).lower()+"\nkey.rusty.menu:key.keyboard.m\n"+"".join(f"mod.{m}:false\n" for m in other))
report={"binary":str(probe.BIN),"sha256":hashlib.sha256(probe.BIN.read_bytes()).hexdigest(),"script_sha256":hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),"offscreen":True,"rusty_enabled":not a.disable_mod,"resolution_requested":[960,540],"render_distance":4,"fps_cap":60,"samples":[],"limitations":["Short local offscreen samples, not long-session or reference-client benchmarks","Comparisons across different Horizon revisions would confound mod-specific attribution","The native Rusty menu keeps single-player simulation live; the host Options menu has a different pause policy","Legacy optional compiled mods are disabled through normal settings if present","The private fixture explicitly rebinds the menu to M for reproducible input"]}
def launch():
 (home/"options.txt").write_text('pauseOnLostFocus:false\nmaxFps:60\nrenderDistance:4\ntutorialStep:none\ninactivityFpsLimit:"minimized"\n',encoding="utf-8")
 env=dict(os.environ)
 for key in ("HORIZON_CONTROL","HORIZON_HOME","HORIZON_LAUNCHER_HOME"):
  env.pop(key,None)
 for key,folder in {"HOME":"home","USERPROFILE":"home","APPDATA":"appdata","LOCALAPPDATA":"localappdata","HORIZON_LAUNCHER_HOME":"launcher","XDG_DATA_HOME":"xdg-data","XDG_CONFIG_HOME":"xdg-config","XDG_CACHE_HOME":"xdg-cache","XDG_RUNTIME_DIR":"runtime","TEMP":"temp","TMP":"temp"}.items():
  target=home/folder;target.mkdir(exist_ok=True);env[key]=str(target.resolve())
 env.update(HORIZON_HOME=str(home.resolve()),HORIZON_NO_AUDIO="1",HORIZON_LAUNCHER_NO_SELF_UPDATE="1")
 command=[str(probe.BIN),"--resources",str(probe.RES),"--game-dir",str(home.resolve()),"--home",str(home.resolve()),"--control","--control-socket",str((home/"control.json").resolve()),"--offscreen","--width","960","--height","540","--render-distance","4","--world","BenchmarkVanillaFixture","--seed","42","--creative","--name","RustyPerfQA"]
 return subprocess.Popen(command,cwd=home.resolve(),env=env,stdout=(home/"process.log").open("wb"),stderr=subprocess.STDOUT,stdin=subprocess.DEVNULL,creationflags=getattr(subprocess,"CREATE_NO_WINDOW",0))
report.update(scan_cache_validation_required=a.require_scan_cache,schema=1,terrain="vanilla",seed=42,inactivity_limit_requested="minimized",daylight_rule_requested="advance_time:false",weather_rule_requested="advance_weather:false",warmup_seconds=15)
process=launch()
try:
 print("started",a.label,process.pid,flush=True)
 deadline=time.monotonic()+180
 while True:
  if process.poll() is not None:raise RuntimeError("Owned benchmark process exited during startup")
  try:
   startup=probe.command(name,"status",5)
   if startup.get("player") and startup.get("chunks",0)>=9 and startup.get("screen")=="playing":break
  except (OSError,ValueError,RuntimeError):pass
  if time.monotonic()>deadline:raise RuntimeError("Owned benchmark startup deadline expired")
  time.sleep(.5)
 for cmd in ["chat /tp @s 8.5 96 3.5","look 0 0","chat /gamerule advance_time false","chat /gamerule advance_weather false","chat /time set noon","chat /weather clear"]:probe.command(name,cmd)
 probe.settled(name);time.sleep(15)
 probe.command(name,"look 0 0");time.sleep(.5)
 initial=probe.command(name,"status")
 assert initial.get("yaw")==0 and initial.get("pitch")==0,"Camera did not match requested yaw/pitch"
 observed=psutil.Process(process.pid)
 cases=[("normal",None)] if a.disable_mod else [("normal",None),("xray","key x tap"),("menu","key m tap")]
 for case,command in cases:
  if command:probe.command(name,command)
  time.sleep(2);probe.settled(name)
  until=time.monotonic()+90
  while time.monotonic()<until:
   state={"disabled":True,"pending":0} if a.disable_mod else probe.command(name,"rusty-status")
   if state.get("pending",0)==0:break
   time.sleep(.2)
  assert state.get("pending",0)==0,"scan failed to settle before benchmark"
  cache_before=state.get("scan_cache") if case=="xray" else None
  if case=="xray" and a.require_scan_cache:
   assert isinstance(cache_before,dict) and isinstance(cache_before.get("reused"),int),"Required scan-cache telemetry is unavailable"
  for i in range(a.samples):
   before=observed.cpu_times();start=time.monotonic();result=probe.command(name,"bench 5",15);wall=time.monotonic()-start;after=observed.cpu_times()
   item={"case":case,"sample":i,"wall_seconds":wall,"cpu_seconds":(after.user+after.system)-(before.user+before.system),"rss_bytes":observed.memory_info().rss,"native":result,"status":probe.command(name,"status"),"rusty":({"disabled":True} if a.disable_mod else probe.command(name,"rusty-status"))}
   report["samples"].append(item);(probe.LIVE/"measurements.json").write_text(json.dumps(report,indent=2)+"\n");print(case,i,"measured",flush=True)
  if case=="xray" and isinstance(cache_before,dict):
   cache_after=probe.command(name,"rusty-status").get("scan_cache")
   assert isinstance(cache_after,dict) and cache_after["reused"]>cache_before["reused"],"Stationary Xray did not reuse native scan aggregation"
   report["scan_cache_validation"]={"passed":True,"stationary_case":"xray","before":cache_before,"after":cache_after,"reused_delta":cache_after["reused"]-cache_before["reused"]}
  probe.shot(name,case)
  report["options_readback"]=(home/"options.txt").read_text()
finally:
 try:probe.command(name,"quit",15)
 except Exception:process.terminate()
 try:process.wait(15)
 except Exception:process.kill();process.wait()
 (home/"control.json").unlink(missing_ok=True)
 report["owned_process_stopped"]=process.poll() is not None
 (probe.LIVE/"measurements.json").write_text(json.dumps(report,indent=2)+"\n")
