#!/usr/bin/env python3
"""Check task geometry and production clipboard extraction in headless Chromium.
Optionally pass an inspect JSON from this worktree's development CLI.
"""
import html
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
source = re.sub(r'<script\b[^>]*>.*?</script>', '', (ROOT / 'ui/index.html').read_text(), flags=re.S)
source = source.replace('<link rel="stylesheet" href="style.css">', '<style>' + (ROOT / 'ui/style.css').read_text() + '</style>')
app = (ROOT / 'ui/app.js').read_text()
copy = app[app.index('function currentPlainText()'):app.index('async function copyNote()')]
if len(sys.argv) > 1:
    fixture = json.loads(Path(sys.argv[1]).read_text())['note']['html']
else:
    fixture = '<ul class="task-list">' + ''.join(f'<li class="task-item"><span class="task-checkbox" role="img" aria-label="{label}" data-checked="{state}">{glyph}</span> {label}</li>' for state, glyph, label in [('false', '☐', '미완료'), ('true', '☑', '완료')]) + '</ul>'
checks = r'''
const $ = id => document.getElementById(id);
let latestNote = {html: FIXTURE};
const rows=[];
try {
 for(const size of [12,14,16,18]) for(const [theme,bg,fg] of [['white','#f8f8f8','#2d2d2d'],['gray','#dfe2dd','#2c2f7e'],['black','#262627','#dfe2dd'],['color','#dfe2dd','#2c2f7e']]) {
  document.documentElement.style.setProperty('--font-size',size+'px');
  document.documentElement.style.setProperty('--background',bg);
  document.documentElement.style.setProperty('--foreground',fg);
  for(const kind of ['', 'added', 'modified', 'removed']) {
   $('content').innerHTML=latestNote.html;
   const symbols=[...document.querySelectorAll('.task-checkbox')];
   if(symbols.length<2)throw Error('missing task fixture');
   if(kind)for(const item of document.querySelectorAll('.task-item'))item.innerHTML='<mark class="change-'+kind+'">'+item.innerHTML+'</mark>';
   // Mark wrapping replaced the nodes; query again below.
   const nodes=[...document.querySelectorAll('.task-checkbox')];
   const geometry=()=>nodes.map(e=>{const r=e.getBoundingClientRect(),li=e.closest('li').getBoundingClientRect(),range=document.createRange();range.setStartAfter(e);range.setEnd(e.parentNode,e.parentNode.childNodes.length);const text=range.getBoundingClientRect();return {width:r.width,height:r.height,row:li.height,x:r.x,baseline:r.bottom-li.top,textX:text.x,textY:text.y-li.top,ink:getComputedStyle(e,'::before').borderColor};});
   const before=geometry();
   for(const e of nodes){const checked=e.dataset.checked!=='true';e.dataset.checked=String(checked);e.textContent=checked?'☑':'☐';}
   const after=geometry();
   if(JSON.stringify(before)!==JSON.stringify(after))throw Error('state moved geometry '+JSON.stringify({size,theme,kind,before,after}));
   if(before.some(e=>e.width!==before[0].width||e.height!==before[0].height))throw Error('unequal outer size');
   const copied=currentPlainText();
   if(!copied.includes('☐ 미완료')||!copied.includes('☑ 완료'))throw Error('clipboard glyph lost '+copied);
   rows.push({size,theme,kind:kind||'plain',...before[0],copy:copied});
  }
 }
 document.body.innerHTML='<pre id="result">'+JSON.stringify({ok:true,rows}).replaceAll('<','&lt;')+'</pre>';
} catch(e) {document.body.innerHTML='<pre id="result">'+JSON.stringify({error:e.message})+'</pre>';}
'''.replace('FIXTURE', json.dumps(fixture))
with tempfile.TemporaryDirectory(prefix='gibbon140-layout-') as temp:
    page=Path(temp)/'check.html'
    page.write_text(source.replace('</body>','<script>'+copy+checks+'</script></body>'))
    result=subprocess.run([CHROME,'--headless','--disable-gpu','--no-first-run','--user-data-dir='+temp+'/profile','--dump-dom',page.as_uri()],text=True,capture_output=True,timeout=30)
    assert result.returncode == 0, result.stderr
    match=re.search(r'<pre id="result">(.*?)</pre>',result.stdout,re.S)
    assert match,result.stderr
    report=json.loads(html.unescape(match.group(1)))
    assert report.get('ok'),report
    print(json.dumps(report,ensure_ascii=False,indent=2))
