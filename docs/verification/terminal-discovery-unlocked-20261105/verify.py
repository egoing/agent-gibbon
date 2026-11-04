"""Validate recorded external evidence; does not operate the app."""
import json
from pathlib import Path
P=Path(__file__).resolve().parent
load=lambda n:json.loads((P/n).read_text())
run=load('run.json');instance=run['dev']['id'];assert instance=='dev-40ca7f182ba9bf0b29e94b20'
for a in run['actions']:assert a['response']['ok'] and a['response']['instance']==instance
for key in ['current_first','current_cached','limited_first','limited_cached']:assert run[key]['inspect']['running'] and run[key]['inspect']['instance']==instance
current=run['current_first']['inspect'];resized=load('current-resize-inspect.json');hidden=load('current-hidden-inspect.json');restored=load('current-restored-inspect.json')
assert current['pty_size']=={'cols':89,'rows':28};assert resized['pty_size']=={'cols':107,'rows':34}
assert not hidden['visible'] and restored['visible']
assert current['pid']==resized['pid']==hidden['pid']==restored['pid']
assert resized['position']==hidden['position']==restored['position']
assert resized['size']==hidden['size']==restored['size']
for label,key,pid in [('current-ready','current_first',50183),('current-restored','current_first',50183),('limited-ready','limited_first',62412),('limited-cache-visible','limited_cached',62412),('slow-result','limited_first',62412)]:
 r=load(label+'.json');s=r['result']['snapshot'];assert r['ok'] and s['app']['pid']==pid and s['window']['title'].endswith(' — '+run[key]['id']);assert r['result']['screenshotStatus']['metadata']['windowId']==s['window']['id']
 # Cross-check exact title + ID + process in independent native window list.
 native=load('current-native.json' if pid==50183 else 'limited-cache-native.json')
 assert any(w.get('kCGWindowNumber')==s['window']['id'] and w.get('kCGWindowName')==s['window']['title'] and w.get('kCGWindowOwnerPID')==pid for w in native['windows'])
shell=[json.loads(l) for l in (P/'limited-cache-shell-events.jsonl').read_text().splitlines()];assert [v['kind'] for v in shell]==['begin','end'];assert shell[0]['args'][0]=='-lc'
r=load('responsive.json');begin,end=r['shell_events'];assert begin['kind']=='begin' and end['kind']=='end' and begin['pid']==end['pid'];assert end['at']-begin['at']>=2.1
oldpid=run['limited_first']['inspect']['pid'];events=[e for e in r['fake_events'] if e['pid']==oldpid and begin['at']<e['at']<end['at']]
assert any(e['kind']=='input' and e['hex']=='31' for e in events);assert any(e['kind']=='input' and e['hex']=='0d' for e in events);assert any(e['kind']=='echo' and e['text']=='SLOW17_1' for e in events);assert any(e['kind']=='size' and e['cols']==107 and e['rows']==34 for e in events)
during=[a for a in r['inspects'] if begin['at']<a['start_at']<a['end_at']<end['at']];assert during and all(a['response']['ok'] and a['response']['instance']==instance and a['response']['pid']==oldpid and a['response']['running'] for a in during)
assert max(a['seconds'] for a in r['inspects'])<.2
fake=[json.loads(l) for l in (P/'fake-events.jsonl').read_text().splitlines()];assert any(e['kind']=='echo' and e['pid']==current['pid'] and e['text']=='17' for e in fake);assert sum(e['kind']=='start' and e['pid']==current['pid'] for e in fake)==1
print('PASS instance, native provenance, ECHO, resize, hide/restore, cache shell count; concurrent inspect count',len(r['inspects']),'wholly during discovery',len(during),'max ms',round(max(a['seconds'] for a in r['inspects'])*1000,3))
