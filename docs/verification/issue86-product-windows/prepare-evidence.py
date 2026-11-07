"""자기 시험 창 내부만 공개한다. 픽셀 자르기 외 변경/합성/확대는 하지 않는다."""
from pathlib import Path
from PIL import Image
import hashlib,json,re
root=Path(__file__).resolve().parent
manifest=root/'screenshot-provenance.json'
items=json.loads(manifest.read_text(encoding='utf-8')) if manifest.exists() else []
published={i['published'] for i in items};redactions=[]
def publish(source,dest,remove=False):
    relative=str(dest.relative_to(root))
    if relative in published:return
    with Image.open(source) as im:
        box=(12,2,im.width-12,im.height-12);im.crop(box).save(dest)
    items.append(dict(source=str(source),source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),published=relative,crop=box,operation='crop only; no rescaling'))
    published.add(relative)
    if remove:source.unlink()
for run in sorted(root.glob('run-*')):
    for p in run.glob('*.json'):
        raw=p.read_bytes()
        try:data=json.loads(raw.decode('utf-16') if raw[:2] in (b'\xff\xfe',b'\xfe\xff') else raw.decode('utf-8-sig'))
        except (ValueError,UnicodeError):continue
        if not isinstance(data,dict):continue
        snapshot=data.get('result',{}).get('snapshot',{});tree=snapshot.get('treeText','')
        unsafe=any('전환 버튼' in line and 'ISSUE86-PRODUCT' not in line for line in tree.splitlines())
        if unsafe:
            snapshot['treeText']='Unfiltered app/window labels omitted for privacy; no screenshot published.'
            data['privacy_redaction']=True
            p.write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8');redactions.append(str(p.relative_to(root)))
        shot=data.get('result',{}).get('screenshot')
        # Overlay captures are full screen; publish only separately reviewed own-fixture crops.
        if shot and shot.get('path') and not unsafe and not data.get('privacy_redaction') and 'overlay' not in p.stem:
            source=Path(shot['path'])
            if source.is_file():publish(source,run/(p.stem+'-client.png'))
    for p in list(run.glob('*.png')):
        if p.stem.endswith('-client') or p.stem.endswith('-crop'):continue
        publish(p,p.with_name(p.stem+'-client.png'),True)
manifest.write_text(json.dumps(items,indent=2),encoding='utf-8')
prior=root/'privacy-redactions.json';old=json.loads(prior.read_text()) if prior.exists() else []
prior.write_text(json.dumps(sorted(set(old+redactions)),indent=2),encoding='utf-8')
print(json.dumps(dict(crops=len(items),redacted_files=len(redactions))))
