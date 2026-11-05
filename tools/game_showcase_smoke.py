"""Reproduce the authored native mine and bounded Fullbright pixel comparison.

Use a freshly built native executable and legal prepared resources. Every run
creates isolated player data and a new offline world. --desktop-hold leaves
that owned window open for manual display checks; its observations require
separate review. See docs/screenshots/README.md for published capture provenance.
"""
# Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
import argparse,json,sys,time,subprocess
from pathlib import Path
from game_feature_smoke import FeatureSmoke,pixel_difference
from game_gui_smoke import digest
p=argparse.ArgumentParser()
for n in ['binary','resources','output']:p.add_argument('--'+n,required=True,type=Path)
p.add_argument('--width',type=int,default=1280);p.add_argument('--height',type=int,default=720);p.add_argument('--startup-timeout',type=float,default=240)
p.add_argument('--desktop-hold',action='store_true')
a=p.parse_args();s=FeatureSmoke(a);original_popen=subprocess.Popen
if a.desktop_hold:
 def visible(command,*args,**kwargs):
  if isinstance(command,list) and command and str(command[0])==str(s.binary):command=[v for v in command if v!='--offscreen']
  return original_popen(command,*args,**kwargs)
 subprocess.Popen=visible
s.report.update(passed=False,suite='native-mine-showcase',driver_sha256=digest(Path(__file__)),claims='Bounded standalone Fullbright pixel comparison and actual native mine/menu captures; optional desktop observations require separate review.')
s.report['observations']=[]
try:
 s.launch();s.command('options-set guiScale 2');s.key('key.keyboard.right.shift');s.wait_for(s.ui,lambda v:v['open'] and bool(v['hits']),'showcase menu');s.search('xray');s.click('Select("xray")');s.wait_for(s.config,lambda c:c['schema']==1,'initial profile');s.key('key.keyboard.escape')
 s.load_area(-8,-8,32,32,y=80)
 for c in ['gamerule advance_time false','gamerule advance_weather false','gamerule spawn_mobs false','time set noon','weather clear','fill -8 60 -8 32 76 32 stone','fill 8 66 0 12 70 20 air','fill 8 65 0 12 65 20 cobbled_deepslate','fill 7 66 0 7 70 20 andesite','fill 13 66 0 13 70 20 granite','fill 4 68 7 5 69 8 diamond_ore','fill 15 68 11 16 69 12 gold_ore','fill 9 68 23 10 69 24 iron_ore','fill 10 63 6 11 63 7 redstone_ore','fill 5 66 15 6 67 16 copper_ore','setblock 9 66 3 torch','setblock 11 66 10 torch','setblock 9 66 17 torch','tp @s 10.5 66 2.5 0 8','gamemode survival']:
  s.server(c)
 s.wait_for(lambda:s.command('block 10 65 2'),lambda b:b['loaded'] and b['block']=='minecraft:cobbled_deepslate','showcase floor')
 s.wait_for(lambda:s.command('state'),lambda v:v['player']['on_ground'] and abs(v['player']['position'][1]-66)<.01,'grounded showcase body')
 s.command('look 0 8');s.wait_render_ready();time.sleep(12)
 pose=s.camera_pose();s.set_hud_hidden(True);normal=s.photo('01-mine-normal',pose)
 s.key('key.keyboard.b');s.wait_for(s.state,lambda v:v['fullbright'] and not v['xray'],'standalone Fullbright on')
 bright=s.photo('01b-mine-fullbright-only',pose);change=pixel_difference(normal,bright)
 s.require(change['changed_fraction']>.02,'Standalone Fullbright did not change the known underground view')
 s.key('key.keyboard.b');s.wait_for(s.state,lambda v:not v['fullbright'] and not v['xray'],'standalone Fullbright restored')
 restored=s.photo('01c-mine-normal-restored',pose);restoration=pixel_difference(normal,restored)
 s.require(restoration['changed_fraction']<=.01,'Standalone Fullbright did not restore the known underground view')
 s.report['standalone_fullbright_pixels']={'enabled':change,'disabled_restoration':restoration,'fixed_camera':pose}
 s.passed('Standalone Fullbright visibly brightens the underground scene and restores normal lighting',enabled=change,disabled_restoration=restoration)
 s.key('key.keyboard.x');s.wait_for(s.state,lambda v:v['xray'],'xray on');scan=s.wait_for(s.state,lambda v:v['pending']==0 and v['targets']>=1,'settled bounded showcase scan',timeout=120);s.report['showcase_scan']=scan;s.photo('02-mine-textured-xray',pose)
 # The terrain comparison stays above; the interface photograph uses an
 # ordinary mine background so its controls remain easy to read.
 s.key('key.keyboard.x');s.wait_for(s.state,lambda v:not v['xray'] and not v['fullbright'],'ordinary terrain before menu capture')
 s.command('options-set chatVisibility 2');s.set_hud_hidden(False)
 s.key('key.keyboard.right.shift');s.wait_for(s.ui,lambda v:v['open'] and bool(v['hits']),'showcase GUI');s.search('');s.top_details();s.shot('03-classic-live-menu')
 s.search('block textures');s.click('Select("xray")');s.hit('Numeric("texture_visibility",',pane='detail');s.shot('04-search-texture-controls')
 s.key('key.keyboard.escape');s.wait_for(s.ui,lambda v:not v['open'],'showcase close')
 if a.desktop_hold:
  s.key('key.keyboard.right.shift');s.wait_for(s.ui,lambda v:v['open'] and bool(v['hits']),'desktop supplement menu')
  print('DESKTOP_READY',flush=True)
  for line in sys.stdin:
   request=json.loads(line);op=request['op']
   if op=='quit':break
   if op=='observe':
    result={'status':s.command('status'),'rusty':s.state(),'ui':s.ui()};s.report['observations'].append({'name':request['name'],'value':result})
   elif op=='command':result=s.command(request['line'])
   elif op=='photo':s.photo(request['name']);result={'captured':request['name']}
   else:raise ValueError('Unknown desktop supplement operation')
   print('REPLY '+json.dumps(result),flush=True)
 s.report['passed']=True;s.report['scope']='Unedited actual native captures in one disposable authored mine. No account or existing world loaded. This visual showcase preserves default finite scan/context budgets, may show their reported partial state, and does not assert complete target coverage.'
finally:
 subprocess.Popen=original_popen
 s.finish()
