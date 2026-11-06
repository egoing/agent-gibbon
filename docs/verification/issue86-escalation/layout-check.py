"""Actual Chromium layout regression; stdlib only, no foreground GUI input.

Usage: python docs/verification/issue86-escalation/layout-check.py --label before
Uses the current HTML and styles, replacing only the native IPC bootstrap with
synthetic public example rows. Browser metrics, not CSS strings, decide pass/fail.
"""
import argparse
import html as html_module
import json
import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
CHECK = r"""
const row = (text) => { const b = document.createElement('button');
  b.className = 'select-row'; b.textContent = text; return b; };
document.querySelector('#apps').append(row('예시 편집기'), row('다른 데스크톱 앱'));
document.querySelector('#windows').append(row('작업 메모 — 예시 편집기'));
document.querySelector('#region').textContent = '선택 영역: 220, 180 · 640×420 px';
document.querySelector('#run-task').disabled = false;
const results = {viewport: [innerWidth, innerHeight], dpr: devicePixelRatio, buttons: [], errors: []};
const buttons = ['run-task', 'refresh', 'cancel'].map(id => document.getElementById(id));
for (const b of buttons) {
  b.scrollIntoView({block: 'center'});
  const r = b.getBoundingClientRect();
  const range = document.createRange(); range.selectNodeContents(b);
  const lines = [...range.getClientRects()];
  results.buttons.push({id: b.id, text: b.textContent, width: r.width, height: r.height,
    left: r.left, right: r.right, top: r.top, bottom: r.bottom, textLines: lines.length});
  if (r.width < 64 || r.height < 32 || lines.length !== 1 ||
      lines.some(t => t.left < r.left || t.right > r.right || t.top < r.top || t.bottom > r.bottom) ||
      r.left < 0 || r.right > innerWidth || r.top < 0 || r.bottom > innerHeight)
    results.errors.push(b.id + ': label/target clipped, wrapped, undersized or unreachable');
}
for (let i=0; i<buttons.length; i++) for (let j=i+1; j<buttons.length; j++) {
  const a=buttons[i].getBoundingClientRect(), b=buttons[j].getBoundingClientRect();
  if (a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom)
    results.errors.push('overlapping buttons');
}
results.wrapped = new Set(buttons.map(b=>b.offsetTop)).size > 1;
if (innerWidth <= 240 && !results.wrapped) results.errors.push('narrow viewport did not wrap actions');
if (document.querySelector('#welcome').scrollWidth > document.querySelector('#welcome').clientWidth)
  results.errors.push('horizontal overflow');
results.pass = results.errors.length === 0;
const pre=document.createElement('pre'); pre.id='layout-result'; pre.textContent=JSON.stringify(results);
parent.document.body.append(pre);
"""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--label', required=True)
    parser.add_argument('--ref', help='Read all UI assets from this git revision')
    parser.add_argument('--browser', default=r'C:\Program Files\Google\Chrome\Application\chrome.exe')
    args = parser.parse_args()
    def asset(name):
        if args.ref:
            return subprocess.check_output(['git', 'show', f'{args.ref}:ui/{name}'], cwd=ROOT).decode('utf-8')
        return (ROOT / 'ui' / name).read_text(encoding='utf-8')
    html = asset('external-apps.html')
    html = re.sub(r'<link rel="stylesheet" href="([^"]+)">',
                  lambda m: '<style>' + asset(m[1]) + '</style>', html)
    html = html.replace('<script src="external-apps.js"></script>', '<script>' + CHECK + '</script>')
    all_results = []
    with tempfile.TemporaryDirectory(prefix='issue86-headless-') as temp:
        fixture = Path(temp) / 'external-apps.html'
        for width, height in [(480, 460), (384, 368), (320, 240), (240, 240)]:
            # Chrome's native headless window has a minimum width/titlebar inset.
            # A frame gives the actual application document the exact viewport.
            fixture.write_text('<!doctype html><meta charset="utf-8"><style>body{margin:0;background:#aaa}pre{font:12px monospace;white-space:pre-wrap}</style>'
                               + f'<iframe style="border:0;width:{width}px;height:{height}px" srcdoc="'
                               + html_module.escape(html, quote=True) + '"></iframe>', encoding='utf-8')
            shot = OUT / f'{args.label}-{width}x{height}.png'
            cmd = [args.browser, '--headless', '--disable-gpu', '--no-first-run',
                   '--no-default-browser-check', '--disable-background-networking',
                   '--disable-component-update', '--disable-sync', '--disable-extensions',
                   '--no-proxy-server', '--host-resolver-rules=MAP * ~NOTFOUND',
                   '--hide-scrollbars', '--force-device-scale-factor=1.25',
                   f'--user-data-dir={temp}/profile-{width}', '--window-size=800,800',
                   '--dump-dom', f'--screenshot={shot}', fixture.as_uri()]
            run = subprocess.run(cmd, capture_output=True, timeout=60,
                                 creationflags=subprocess.CREATE_NO_WINDOW)
            (OUT / f'{args.label}-{width}x{height}-browser.log').write_bytes(run.stderr)
            dom = run.stdout.decode('utf-8')
            match = re.search(r'<pre id="layout-result">(.*?)</pre>', dom, re.S)
            if run.returncode != 0 or not match:
                raise RuntimeError(f'browser failed: exit={run.returncode}, metrics={bool(match)}')
            result = json.loads(match[1])
            if result['viewport'] != [width, height]:
                result['errors'].append('actual viewport differs from requested viewport')
                result['pass'] = False
            result['requestedViewport'] = [width, height]
            result['screenshot'] = shot.name
            all_results.append(result)
    (OUT / f'{args.label}-layout.json').write_text(json.dumps(all_results, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps(all_results, ensure_ascii=False, indent=2))
    return 0 if all(r['pass'] for r in all_results) else 1


if __name__ == '__main__':
    raise SystemExit(main())
