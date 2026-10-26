#!/usr/bin/env python3
"""Exercise rendered clipboard text and geometry in headless Chromium."""
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
app = (ROOT / 'ui/app.js').read_text()
copy = app[app.index('function currentPlainText()'):app.index("$('copy').addEventListener('click', copyNote);")]
checks = r'''
const $ = id => document.getElementById(id);
let latestNote = { html: '<h1>최신 제목</h1><p>현재 <strong>본문</strong><br>다음 줄</p><ul><li>첫 항목</li><li>둘째 항목</li></ul><pre><code>const x = 1;\n  x++;</code></pre><table><tr><td>A</td><td>B</td></tr></table>' };
let editing = false, copyFeedback, written, imageCopies = 0, error;
let expiry;
const setTimeout = callback => { expiry = callback; return 1; }, clearTimeout = () => {};
window.__TAURI__ = { clipboardManager: { async writeText(text) { written = text; } } };
async function invoke(name) { if (name !== 'copy_note_image') throw Error(name); imageCopies++; }
function failure(value) { error = value; }
function check(ok, message) { if (!ok) throw Error(message); }
async function run() {
  const results = [];
  try {
    $('content').innerHTML = latestNote.html;
    const expected = $('content').innerText;
    $('content').innerHTML = '<del>삭제된 이전 내용</del>' + latestNote.html;
    const visible = $('content').innerHTML;
    $('diff-actions').hidden = $('clear-diff').hidden = false;
    const before = $('viewport').getBoundingClientRect().height;
    await copyNote();
    check(!error && written === expected && !written.includes('삭제된 이전'), 'current rendered text format differs');
    check($('content').innerHTML === visible && !$('diff-actions').hidden, 'copy changed pending diff');
    check($('viewport').getBoundingClientRect().height === before, 'feedback resized viewport');
    const b = $('clear-diff').getBoundingClientRect();
    check(document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2).closest('button') === $('clear-diff'), 'feedback blocks confirmation');
    check(!$('copy-status').hidden, 'success not shown'); expiry();
    check($('copy-status').hidden, 'success did not expire');
    results.push('latest rendered headings, paragraphs, list, code and table; diff and footer preserved; feedback expires');
    latestNote.capture = { width: 300, height: 150 };
    document.body.classList.add('image-note');
    const rect = $('content').getBoundingClientRect();
    await copyNote();
    const after = $('content').getBoundingClientRect();
    check(imageCopies === 1 && !error, 'native image copy not invoked');
    check(rect.width === after.width && rect.height === after.height, 'image geometry changed');
    results.push('native image path and image geometry preserved');
  } catch (e) { results.push({ error: e.message }); }
  const output = document.createElement('pre'); output.id = 'copy-result'; output.textContent = JSON.stringify(results); document.body.append(output);
}
run();
'''
with tempfile.TemporaryDirectory(prefix='gibbon-copy-layout-') as temp:
    page = Path(temp) / 'copy.html'
    # Declarations precede the extracted production copy functions.
    declarations, execution = checks.split('async function run()', 1)
    page.write_text(source.replace('</body>', '<script>' + declarations + copy + 'async function run()' + execution + '</script></body>'))
    run = subprocess.run([CHROME, '--headless', '--disable-gpu', '--no-first-run', '--user-data-dir=' + temp + '/profile', '--window-size=640,480', '--dump-dom', page.as_uri()], text=True, capture_output=True, check=True, timeout=30)
    match = re.search(r'<pre id="copy-result">(.*?)</pre>', run.stdout, re.S)
    assert match, run.stderr
    for result in json.loads(html.unescape(match.group(1))):
        assert isinstance(result, str), result
        print('PASS:', result)
