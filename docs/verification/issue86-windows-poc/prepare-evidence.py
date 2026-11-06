"""공개 증거에서 전용 창의 바깥 그림자에 비친 기존 앱 픽셀만 제외한다.
그 외 픽셀 변경/합성/확대는 하지 않으며 raw.jsonl은 수정하지 않는다.
Usage: python prepare-evidence.py <run-folder>
"""
import hashlib, json, pathlib, sys
from PIL import Image

root=pathlib.Path(sys.argv[1]).resolve()
items=[]
sources=[root/name for name in ("01-initial-A.png","02-position-A.png","07-restored-A.png")]
official=json.loads((root/"orca-exposure-capture.stdout.txt").read_text(encoding="utf-8-sig"))["result"]["screenshot"]["path"]
sources.append(pathlib.Path(official))
for source in sources:
    name="00-orca-native-capture-client.png" if source.parent!=root else source.stem+"-client.png"
    destination=root/name
    with Image.open(source) as image:
        assert image.size==(640,420)
        image.crop((12,2,628,408)).save(destination)
    items.append(dict(source=str(source),published=name,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),crop_box=[12,2,628,408],operation="crop only; no rescaling"))
    # Only the three known PoC-generated desktop crops are removed, never the Orca temp image.
    if source.parent==root: source.unlink()
(root/"screenshot-provenance.json").write_text(json.dumps(items,ensure_ascii=False,indent=2),encoding="utf-8")
manifest={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(root.iterdir()) if p.is_file() and p.name!="sha256.json"}
(root/"sha256.json").write_text(json.dumps(manifest,indent=2),encoding="utf-8")
