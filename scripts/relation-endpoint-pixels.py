#!/usr/bin/env python3
"""Verify visible SVG endpoints against actual PID-owned memo/window bounds on macOS."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile
from PIL import Image
parser=argparse.ArgumentParser()
parser.add_argument('pid',type=int)
parser.add_argument('source')
parser.add_argument('target')
parser.add_argument('--native',default='/tmp/gibbon168-native/native')
parser.add_argument('--output',required=True)
parser.add_argument('--expect-detached',action='store_true')
args=parser.parse_args()
def windows():
    return json.loads(subprocess.check_output([args.native,str(args.pid)],text=True))['windows']
def identify(rows,id):
    return next(w for w in rows if w.get('kCGWindowName','').endswith(' — '+id))
rows=windows();a=identify(rows,args.source);b=identify(rows,args.target)
overlay=identify(rows,'relation-prototype-0')
a=a['kCGWindowBounds'];b=b['kCGWindowBounds'];o=overlay['kCGWindowBounds']
ax,ay,aw,ah=[a[k] for k in ['X','Y','Width','Height']]
bx,by,bw,bh=[b[k] for k in ['X','Y','Width','Height']]
gx=max(bx-ax-aw,ax-bx-bw);gy=max(by-ay-ah,ay-by-bh)
assert gx>=0 or gy>=0,'overlapping notes intentionally have no connector'
if gx>=0 and gx>=gy:
    sign=1 if bx+bw/2>=ax+aw/2 else -1
    endpoints=[[ax+aw/2+sign*aw/2,ay+ah/2],[bx+bw/2-sign*bw/2,by+bh/2]]
else:
    sign=1 if by+bh/2>=ay+ah/2 else -1
    endpoints=[[ax+aw/2,ay+ah/2+sign*ah/2],[bx+bw/2,by+bh/2-sign*bh/2]]
out=Path(args.output);out.parent.mkdir(parents=True,exist_ok=True)
subprocess.run(['screencapture','-x','-o','-l',str(overlay['kCGWindowNumber']),str(out)],check=True)
image=Image.open(out).convert('RGBA');sx=image.width/o['Width'];sy=image.height/o['Height']
checks=[]
for point in endpoints:
    x=round((point[0]-o['X'])*sx);y=round((point[1]-o['Y'])*sy)
    matches=[]
    for xx in range(max(0,x-4),min(image.width,x+5)):
        for yy in range(max(0,y-4),min(image.height,y+5)):
            r,g,blue,alpha=image.getpixel((xx,yy))
            if alpha>70 and max(abs(r-98),abs(g-101),abs(blue-144))<35:matches.append([xx,yy])
    checks.append({'screen_point':point,'image_point':[x,y],'stroke_present':bool(matches)})
latest=windows()
assert identify(latest,args.source)['kCGWindowBounds']==a and identify(latest,args.target)['kCGWindowBounds']==b,'memo moved during pixel capture; retry'
result={'pid':args.pid,'source':args.source,'target':args.target,'overlay_bounds':o,'image_size':image.size,'checks':checks,'attached':all(c['stroke_present'] for c in checks)}
out.with_suffix('.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(result,ensure_ascii=False))
assert not result['attached'] if args.expect_detached else result['attached'],result
