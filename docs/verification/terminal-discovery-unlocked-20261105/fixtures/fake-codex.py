#!/usr/bin/python3
"""External fake CLI only; no model/network/authentication calls."""
import os, tty, signal, json, fcntl, termios, struct, time
from pathlib import Path
base=Path('/tmp/gibbon17-40ca7f-unlocked')
def event(kind,**data):
 with (base/'fake-events.jsonl').open('a') as f: f.write(json.dumps({'kind':kind,'at':time.time(),'mono_ns':time.monotonic_ns(),'pid':os.getpid(),**data})+'\n')
def size(*_):
 rows,cols,_,_=struct.unpack('HHHH',fcntl.ioctl(0,termios.TIOCGWINSZ,b'\0'*8))
 event('size',rows=rows,cols=cols)
 os.write(1,('\r\nSIZE %sx%s\r\n'%(cols,rows)).encode())
tty.setraw(0); signal.signal(signal.SIGWINCH,size)
event('start',cwd=os.getcwd())
os.write(1,b'\x1b[32mGIBBON17_FINAL_READY\x1b[0m\r\nNative input / resize / hide-restore\r\n')
size(); line=b''
while True:
 b=os.read(0,1)
 if not b: break
 event('input',hex=b.hex())
 if b in [b'\r',b'\n']:
  os.write(1,b'\r\nGIBBON17_ECHO:'+line+b'\r\n');event('echo',text=line.decode('utf-8','replace'));line=b''
 elif b==b'\x03': os.write(1,b'^C\r\n');line=b''
 elif b in [b'\x7f',b'\x08']: line=line[:-1];os.write(1,b'\b \b')
 else: line+=b;os.write(1,b)
