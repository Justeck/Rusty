#!/usr/bin/env python3
"""Real native input in an owned offline world; never loads user settings/worlds."""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import sys
import time
from game_gui_smoke import Smoke, AcceptanceFailure
from game_ui12_smoke import Ui12Smoke, same_native_layout


class HudEditorSmoke(Ui12Smoke):
    def panel(self, name='coordinates'):
        return next(p for p in self.ui()['hud_panels'] if p['id'] == name)

    def gesture(self, start, end):
        self.command(f'drag-at {start[0]:.4f} {start[1]:.4f} {end[0]:.4f} {end[1]:.4f}')
        time.sleep(.25)

    def exercise(self):
        self.deadline=time.monotonic()+self.args.budget
        self.launch();self.command("options-set guiScale 2")
        self.open_browser();self.select('coordinates');self.click('Toggle("coordinates")',pane='detail')
        self.wait_for(self.config,lambda c:c['coords_hud'],'coordinates enabled')
        before=copy.deepcopy(self.config())
        self.click('EditHud',pane='detail')
        self.wait_for(self.ui,lambda s:s.get('hud_editing'),'visual HUD editor')
        self.require(not self.ui()['hits'],'menu controls must not remain clickable')
        p=self.panel();x,y,w,h=p['rect'];start=[x+w/2,y+h/2]
        self.gesture(start,[start[0]+120,start[1]+60])
        q=self.panel();self.require(abs(q['rect'][0]-x-120)<1 and abs(q['rect'][1]-y-60)<1,'drag must track the grabbed point')
        self.require(self.config()==before,'draft drag must not change the saved configuration')
        self.passed('Edit HUD replaces coordinate controls and drags actual overlay geometry without saving')
        x,y,w,h=q['rect'];scale=q['scale']
        self.gesture([x+w,y+h],[x+w+w*.3,y+h+h*.3])
        q=self.panel();self.require(abs(q['scale']/scale-1.3)<.02,'corner must resize proportionally')
        self.require(abs(q['rect'][0]-x)<1 and abs(q['rect'][1]-y)<1,'opposite corner must stay fixed')
        self.capture('01-hud-editor','Actual world view with selected, dragged and resized coordinates overlay and corner handles')
        self.passed('Image-like corner resizing keeps the opposite corner anchored')
        draft=copy.deepcopy(self.ui()['hud_draft'])
        self.key('key.keyboard.enter')
        self.wait_for(self.ui,lambda s:s['open'] and not s['hud_editing'] and s['selected']=='coordinates','Enter returns to same module')
        self.wait_for(self.config,lambda c:same_native_layout(draft,c['client']['hud_layout']),'confirmed persisted layout')
        self.passed('Enter confirms the layout and returns to the same mod menu')
        saved=copy.deepcopy(self.config())
        self.click('EditHud',pane='detail');self.wait_for(self.ui,lambda s:s['hud_editing'],'reopen editor')
        p=self.panel();x,y,w,h=p['rect'];self.gesture([x+w/2,y+h/2],[x+w/2+30,y+h/2+20])
        self.key('key.keyboard.escape')
        self.wait_for(self.ui,lambda s:s['open'] and not s['hud_editing'],'Escape returns to menu')
        self.require(self.config()==saved,'Escape must discard the entire draft')
        self.passed('Escape discards edits without changing features, saved layout or menu selection')
        self.click('EditHud',pane='detail');self.wait_for(self.ui,lambda s:s['hud_editing'],'input safety editor')
        p=self.panel();status=self.panel('status_hud');x,y,w,h=p['rect']
        self.gesture([x+w/2,y+h/2],[status['rect'][0]+w/2,status['rect'][1]+h/2])
        p=self.panel();x,y,w,h=p['rect']
        self.command(f'click-at {x+w/2:.4f} {y+h/2:.4f}')
        self.require(self.ui()['hud_selected']=='coordinates','visible topmost overlay must win overlapping picks')
        self.passed('Overlapping HUD selection follows actual paint order')
        # Native HUD extent is exposed separately from menu-scaled control targets.
        viewport=self.ui()['hud_viewport']
        self.gesture([x+w/2,y+h/2],[3+w/2,viewport[1]-3-h/2])
        p=self.panel();x,y,w,h=p['rect']
        self.gesture([x+w/2,y+h/2],[x+w/2+30,y+h/2-60])
        q=self.panel();self.require(abs(q['rect'][1]-y+60)<1,'bottom-edge overlay must remain draggable outside footer buttons')
        self.passed('Bottom-edge overlay remains selectable and draggable beside footer controls')
        body=self.command('status')['player']['position']
        self.key('key.keyboard.w','down');time.sleep(.4);self.key('key.keyboard.w','up')
        after=self.command('status')['player']['position']
        self.require(body==after,'editor movement key must not move the body')
        self.key('key.keyboard.escape');self.close_browser()
        self.capture('02-confirmed-hud','Confirmed overlay placement in the ordinary game view')
        self.require(same_native_layout(draft,self.config()['client']['hud_layout']),'confirmed layout remains after closing menu')
        self.passed('Editing consumes gameplay movement; confirmed layout remains in ordinary game view')

    def finish(self):
        self.deadline=None
        Smoke.finish(self)
        (self.output/'gui-acceptance.json').replace(self.output/'hud-editor-acceptance.json')


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['binary','resources','output']:p.add_argument('--'+name,required=True,type=Path)
    p.add_argument('--width',type=int,default=1280);p.add_argument('--height',type=int,default=720)
    p.add_argument('--startup-timeout',type=float,default=300);p.add_argument('--budget',type=float,default=900)
    args=p.parse_args();s=None
    try:
        s=HudEditorSmoke(args);s.exercise();s.report['passed']=True;return 0
    except (AcceptanceFailure,OSError,ValueError,KeyError,TypeError,StopIteration,subprocess.SubprocessError) as e:
        if s:
            s.report['passed']=False;s.report['checks'].append({'name':s.phase,'passed':False,'reason':str(e)})
            try:s.shot('failure')
            except Exception:pass
        print('FAIL '+str(e),file=sys.stderr,flush=True);return 1
    finally:
        if s:s.finish()

if __name__=='__main__':raise SystemExit(main())
