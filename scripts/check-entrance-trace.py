#!/usr/bin/env python3
"""Check opt-in entrance JSONL against actual native acknowledgements.

Fails on unintended interruptions, unapplied movement commands, or a final
frame size different from its plan. Pair with open-order-smoke --trace-file to
verify all current destinations settled; this check alone proves readback of
recorded frames. No GUI or app mutations are performed.
"""
import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('trace', type=Path)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--window-prefix', default='')
    args = parser.parse_args()
    rows = [json.loads(line) for line in args.trace.read_text().splitlines() if line.strip()]
    rows = [r for r in rows if r['window'].startswith(args.window_prefix)]
    frames = [r for r in rows if r['event'] == 'frame']
    failures = [r for r in rows if r['event'] in ('position-mismatch', 'queue-error', 'frame-wait-error')]
    unacknowledged = []
    wrong_final_sizes = []
    for row in frames:
        d = row['data']
        actual, requested = d['actual_after'], d['next']
        if actual is None or any(abs(actual[k] - requested[k]) > 2 for k in ('x', 'y')):
            unacknowledged.append(row)
        if d['t'] >= 1 and d['size_after'] != d['flight']['destination']['size']:
            wrong_final_sizes.append(row)
    result = {
        'trace': str(args.trace.resolve()),
        'window_prefix': args.window_prefix,
        'frames': len(frames),
        'windows': sorted({r['window'] for r in rows}),
        'interrupted_frames': len(failures),
        'unacknowledged_positions': len(unacknowledged),
        'wrong_final_sizes': len(wrong_final_sizes),
        'interruption_evidence': failures,
        'unacknowledged_examples': unacknowledged[:8],
        'wrong_size_examples': wrong_final_sizes[:8],
    }
    result['ok'] = bool(frames) and not (failures or unacknowledged or wrong_final_sizes)
    args.report.write_text(json.dumps(result, ensure_ascii=False, indent=2))
    print(json.dumps({k: v for k, v in result.items() if k not in ('windows', 'interruption_evidence', 'unacknowledged_examples', 'wrong_size_examples')}, ensure_ascii=False))
    raise SystemExit(0 if result['ok'] else 1)


if __name__ == '__main__':
    main()
