#!/usr/bin/env python3
"""Check real CSS hit testing and the reserved bottom action area in headless Chrome."""
import html
import json
from pathlib import Path
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
source = (ROOT / 'ui/index.html').read_text()
source = re.sub(r'<script\b[^>]*>.*?</script>', '', source, flags=re.S)
source = source.replace('<link rel="stylesheet" href="style.css">', '<style>' + (ROOT / 'ui/style.css').read_text() + '</style>')
checks = r'''
const results = [];
function check(ok, message) { if (!ok) throw Error(message); }
try {
  const menu = document.getElementById('diff-actions');
  const button = document.getElementById('clear-diff');
  const viewport = document.getElementById('viewport');
  document.getElementById('dev-instance').hidden = false;
  document.getElementById('dev-instance').textContent = 'DEV egoing/diff-clean-when-focus · 861e11d9';
  document.getElementById('content').innerHTML = Array.from({length: 30}, (_, i) => `<p>본문 ${i}: 마지막 줄도 메뉴 위에서 읽을 수 있습니다.</p>`).join('');
  for (const [width, height, font] of [[300, 240, 12], [144, 160, 18], [300, 80, 12]]) {
    document.body.style.width = width + 'px';
    document.body.style.height = height + 'px';
    document.documentElement.style.setProperty('--font-size', font + 'px');
    menu.hidden = button.hidden = false;
    const v = viewport.getBoundingClientRect(), m = menu.getBoundingClientRect(), b = button.getBoundingClientRect();
    check(v.bottom <= m.top, 'menu overlaps the scrolling viewport');
    check(b.left >= m.left && b.right <= m.right && b.bottom <= m.bottom, 'button exceeds its reserved area');
    check(document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2).closest('button') === button, 'button does not receive mouse input');
    viewport.scrollTop = viewport.scrollHeight;
    const last = document.querySelector('#content > :last-child').getBoundingClientRect();
    check(last.bottom <= v.bottom + 1, 'last line cannot scroll clear of the menu');
    const before = v.height;
    menu.hidden = button.hidden = true;
    check(viewport.getBoundingClientRect().height > before, 'hiding menu does not release space');
    viewport.scrollTop = 0;
    results.push(`${width}x${height}, font ${font}: click target, reserved space and last line passed`);
  }
  document.body.style.width = '300px'; document.body.style.height = '240px';
  const edit = document.getElementById('edit'), e = edit.getBoundingClientRect();
  check(document.elementFromPoint(e.x + e.width / 2, e.y + e.height / 2).closest('button') === edit, 'development label covers toolbar');
  menu.hidden = button.hidden = false;
  document.body.classList.add('folded');
  check(menu.getBoundingClientRect().height === 0, 'folded note retains menu');
  results.push('toolbar hit testing and folded menu passed');
} catch (error) { results.push({error: error.message}); }
const output = document.createElement('pre'); output.id = 'layout-result'; output.textContent = JSON.stringify(results); document.body.append(output);
'''
with tempfile.TemporaryDirectory(prefix='gibbon-diff-layout-') as temp:
    page = Path(temp) / 'layout.html'
    page.write_text(source.replace('</body>', '<script>' + checks + '</script></body>'))
    run = subprocess.run([CHROME, '--headless', '--disable-gpu', '--no-first-run', '--user-data-dir=' + temp + '/profile', '--window-size=640,480', '--dump-dom', page.as_uri()], text=True, capture_output=True, check=True, timeout=30)
    match = re.search(r'<pre id="layout-result">(.*?)</pre>', run.stdout, re.S)
    assert match, run.stderr
    results = json.loads(html.unescape(match.group(1)))
    for result in results:
        assert isinstance(result, str), result
        print('PASS:', result)
