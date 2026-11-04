#!/usr/bin/python3
"""Externally timed shell fixture; only -lc/-ilc discovery is simulated."""
import os,sys,json,time
from pathlib import Path
base=Path('/tmp/gibbon17-40ca7f-unlocked')
def event(kind,**data):
 with (base/'shell-events.jsonl').open('a') as f: f.write(json.dumps({'kind':kind,'at':time.time(),'mono_ns':time.monotonic_ns(),'pid':os.getpid(),'args':sys.argv[1:],**data})+'\n')
event('begin'); config=json.loads((base/'shell-config.json').read_text());time.sleep(config['delay'])
print('\nAGENT_GIBBON_PATH='+config['path'],flush=True);event('end')
