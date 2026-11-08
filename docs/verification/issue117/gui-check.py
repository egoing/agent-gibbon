import json,subprocess,time
from pathlib import Path
R=Path.cwd(); O=R/'docs/verification/issue117'; N='/tmp/gibbon117-native'; PID=47454

def native(*a):return subprocess.check_output([N,*map(str,a)],text=True)
def capture(window_id,destination):
 r=json.loads(subprocess.check_output(['orca','computer','get-app-state','--app',f'pid:{PID}','--window-id',str(window_id),'--json'],text=True))
 assert r['ok'] and r['result']['snapshot']['app']['pid']==PID
 assert r['result']['snapshot']['window']['id']==window_id
 import shutil
 shutil.copyfile(r['result']['screenshot']['path'],destination)
 Path(str(destination)+'.json').write_text(json.dumps(r,ensure_ascii=False,indent=2))
def ax():return json.loads(native('ax',PID))
def notice(tree):return next(w for w in tree if w['title']=='Agent Gibbon · 업데이트')
def flat(n):return [n]+[z for c in n.get('children',[]) for z in flat(c)]
def cli():
 d=json.loads(subprocess.check_output([str(R/'scripts/agent-gibbon-dev'),'inspect','--id','issue117-gui-control'],text=True));assert d['ok'] and d['instance']=='dev-36ae4e7398777ce0cdab2feb';return d
base=notice(ax()); targets=[n for n in flat(base) if (n['role']=='AXStaticText' and n['value']=='편집 또는 저장이 끝나면 설치합니다') or (n['role']=='AXButton' and n['title'] in ['나중에','업데이트하고 재시작'])];assert len(targets)==3
records=[]
try:
 for i,t in enumerate(targets):
  native('ctrl','up');native('move',580,480);time.sleep(.25);native('ctrl','down');time.sleep(.15)
  p=t['position'];s=t['size'];x=p['x']+min(s.get('width',0)/2,80);y=p['y']+s.get('height',0)/2
  native('move',x,y,'ctrl');time.sleep(.4)
  current=notice(ax());assert current==base,('notice geometry/content changed',i)
  (O/f'ax-ctrl-{i}.json').write_text(json.dumps(current,ensure_ascii=False,indent=2))
  records.append({'target':t['title'] or t['value'],'pointer':[x,y],'unchanged':True})
  if i==2:
   ws=json.loads(native('windows',PID));w=next(w for w in ws if w.get('kCGWindowBounds',{}).get('X')==600)
   capture(w['kCGWindowNumber'],O/'notice-ctrl.png')
 native('ctrl','up');native('move',580,480);time.sleep(.25)
 before=cli();assert not before['folded'] and not before['focused']
 p=before['position'];s=before['size'];x=(p['x']+s['width']/2)/2;y=(p['y']+s['height']/2)/2
 native('ctrl','down');native('move',x,y,'ctrl');time.sleep(.5)
 folded=cli();assert folded['folded'], 'positive control did not fold'
 (O/'inspect-positive-folded.json').write_text(json.dumps(folded,ensure_ascii=False,indent=2))
 ws=json.loads(native('windows',PID));w=next(w for w in ws if w.get('kCGWindowBounds',{}).get('X')==1200)
 print('Positive screenshot unavailable: Orca excludes 12pt folded window; native CLI/AX geometry retained')
 native('ctrl','up');native('move',580,480);time.sleep(.3)
 p=folded['position'];s=folded['size'];native('ctrl','down');native('move',(p['x']+s['width']/2)/2,(p['y']+s['height']/2)/2,'ctrl');time.sleep(.5)
 restored=cli();assert not restored['folded'];assert restored['size']==before['size'] and restored['position']==before['position']
 (O/'inspect-positive-restored.json').write_text(json.dumps(restored,ensure_ascii=False,indent=2))
 print('PASS: notice text/buttons unchanged; ordinary memo fold/unfold positive control')
 records.append({'positive_fold':True,'unfold_restored':True,'before_size':before['size'],'folded_size':folded['size'],'restored_size':restored['size']})
finally:
 native('ctrl','up');p=json.loads(Path('/tmp/gibbon117-pointer.json').read_text());native('move',p['x'],p['y']);time.sleep(3)
 (O/'native-results.json').write_text(json.dumps({'pid':PID,'targets':records,'front_after':json.loads(native('pointer'))},ensure_ascii=False,indent=2))
 (O/'windows-installed-after.json').write_text(native('windows',95607))
