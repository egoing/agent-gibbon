#!/usr/bin/env python3
"""Same-worktree relation CPU comparison, preserving user memos and relation file."""
import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import uuid
ROOT=Path(__file__).resolve().parents[1]
OUT=Path('/tmp/gibbon168-smooth');OUT.mkdir(exist_ok=True)
def run(*args):return subprocess.check_output(list(map(str,args)),cwd=ROOT,text=True,timeout=40).strip()
info=json.loads(run('node','scripts/dev.mjs','info'))
lib=ctypes.CDLL('/usr/lib/libproc.dylib')
lib.proc_pidpath.argtypes=[ctypes.c_int,ctypes.c_void_p,ctypes.c_uint32]
lib.proc_pid_rusage.argtypes=[ctypes.c_int,ctypes.c_int,ctypes.c_void_p]
pids=[]
for line in run('ps','-axo','pid=').splitlines():
    pid=int(line);buf=ctypes.create_string_buffer(4096)
    if lib.proc_pidpath(pid,buf,len(buf))>0 and buf.value.decode()==info['binary']:pids.append(pid)
assert len(pids)==1,pids
pid=pids[0]
config=Path('/tmp')/f"floatnote-{os.getuid()}-{info['id']}"/'relation-prototype.json'
original=config.read_bytes() if config.exists() else b'[]'
def write(data):
    fd,path=tempfile.mkstemp(dir=config.parent)
    with os.fdopen(fd,'wb') as f:f.write(data)
    os.replace(path,config)
def webpids():
    return {int(s.split()[0]) for s in run('ps','-axo','pid=,comm=').splitlines() if s.rstrip().endswith('/com.apple.WebKit.WebContent')}
def cpu(ids):
    total=0
    for p in ids:
        b=ctypes.create_string_buffer(1024)
        assert lib.proc_pid_rusage(p,2,ctypes.byref(b))==0,p
        total+=(ctypes.c_uint64.from_buffer(b,16).value+ctypes.c_uint64.from_buffer(b,24).value)/1e9
    return total
front=run('swift',ROOT/'scripts/frontmost.swift')
ids=['curve-perf-'+uuid.uuid4().hex[:8]+s for s in ['-a','-b']]
created=[];results=[]
try:
    for source,name in [('design-system-ax.swift','ax'),('relation-motion.swift','motion')]:
        run('swiftc',ROOT/'scripts'/source,'-o',OUT/name)
    write(b'[]');time.sleep(.5)
    titles=[]
    for i,id in enumerate(ids):
        r=json.loads(run(ROOT/'scripts/agent-gibbon-dev','open','--id',id,'--markdown','곡선 성능 검증 '+id));assert r['ok'];created.append(id)
        state=json.loads(run(ROOT/'scripts/agent-gibbon-dev','inspect','--id',id));assert state['instance']==info['id'] and state['note']['markdown']=='곡선 성능 검증 '+id
        title=f"Agent Gibbon [{state['dev_label']}] — {id}";titles.append(title)
        run(OUT/'ax',pid,title,'position',80+620*i,160+140*i)
        run(OUT/'ax',pid,title,'resize',300,180)
    time.sleep(2)
    before=webpids()
    write(json.dumps([{'from':ids[0],'to':ids[1],'label':'참고'}],ensure_ascii=False).encode());time.sleep(2)
    overlay_pids=webpids()-before
    assert len(overlay_pids)==1,overlay_pids
    measured=[pid,*overlay_pids]
    rounds=int(os.environ.get('GIBBON_RELATION_PERF_ROUNDS','1'))
    for sample in range(rounds):
      for case in ['idle','moving']:
          start=[cpu([p]) for p in measured];t=time.monotonic()
          frames=None
          if case=='idle':time.sleep(5)
          else:frames=int(run(OUT/'motion',pid,titles[1],5))
          wall=time.monotonic()-t;used=[cpu([p])-c for p,c in zip(measured,start)]
          results.append({'sample':sample+1,'case':case,'wall_seconds':wall,'cpu_seconds_by_pid':dict(zip(measured,used)),'cpu_percent_one_core':sum(used)/wall*100,'movement_requests':frames})
    print(json.dumps({'info':info,'pid':pid,'overlay_webkit_pids':list(overlay_pids),'memo_ids':ids,'titles':titles,'scope':'Rust app + newly created overlay WebContent; excludes memo WebContents, GPU and WindowServer','results':results},ensure_ascii=False))
finally:
    write(original)
    for id in created:subprocess.run([str(ROOT/'scripts/agent-gibbon-dev'),'close','--id',id],cwd=ROOT,capture_output=True)
    if run('swift',ROOT/'scripts/frontmost.swift')!=front:
        run('swift','-e','import AppKit; let pid=pid_t(CommandLine.arguments.last!)!; if let app=NSRunningApplication(processIdentifier:pid) { app.activate(options:[]) }',front)
        time.sleep(.3)
