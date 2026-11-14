#!/usr/bin/env python3
"""Development-only relation configuration; never starts an app."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('from_id', nargs='?')
parser.add_argument('to_id', nargs='?')
parser.add_argument('--label', default='참고')
parser.add_argument('--clear', action='store_true')
args = parser.parse_args()
if not args.clear and (not args.from_id or not args.to_id):
    parser.error('from_id와 to_id 또는 --clear가 필요합니다')
instance = 'dev-' + hashlib.sha256(str(root).encode()).hexdigest()[:24]
# Match the existing per-worktree local IPC directory. No installed endpoint fallback.
folder = Path('/tmp') / f'floatnote-{os.getuid()}-{instance}'
if not folder.is_dir():
    parser.error('해당 워크트리의 개발 앱을 먼저 실행하세요')
relations = [] if args.clear else [{'from': args.from_id, 'to': args.to_id, 'label': args.label}]
fd, name = tempfile.mkstemp(prefix='relation-', dir=folder)
try:
    with os.fdopen(fd, 'w') as output:
        json.dump(relations, output, ensure_ascii=False)
    os.replace(name, folder / 'relation-prototype.json')
finally:
    if os.path.exists(name):
        os.unlink(name)
print(json.dumps({'instance': instance, 'relations': relations}, ensure_ascii=False))
