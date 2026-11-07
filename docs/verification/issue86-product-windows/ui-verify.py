"""공식 Orca 도구의 실제 입력으로 선택→실행→해제/취소를 검증한다."""
import runpy,pathlib,subprocess,json,re,time,os,sys,traceback
sys.dont_write_bytecode=True
d=runpy.run_path(str(pathlib.Path(__file__).with_name('run-product.py')),run_name='helpers')
globals().update({k:v for k,v in d.items() if not k.startswith('__')})
api('SetProcessDpiAwarenessContext',-4)
root=HERE.parents[2];ctx=json.loads((HERE/'ui-session.json').read_text());a=ctx['owned'][0];b=ctx['owned'][1];checks={};cli=str(root/'scripts/agent-gibbon-dev.cmd')
info=json.loads(subprocess.check_output(['node','scripts/dev.mjs','info'],cwd=root,text=True))
ps='Get-CimInstance Win32_Process -Filter "Name = \'agent-gibbon.exe\'" | Where-Object { $_.ExecutablePath -eq $env:GIBBON_TEST_BINARY } | Select-Object ProcessId,ExecutablePath | ConvertTo-Json -Compress'
server=json.loads(subprocess.check_output(['powershell.exe','-NoProfile','-Command',ps],env={**os.environ,'GIBBON_TEST_BINARY':info['binary']},text=True));pid=server['ProcessId'];app='pid:'+str(pid)
def product(name,*args):
    p=subprocess.run([cli,*args],cwd=root,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=40);value=json.loads(p.stdout)
    log('product',name=name,args=args,exit=p.returncode,response=value);return value
def orca(name,args,tree_safe=False):
    p=subprocess.run(['orca','computer',*args,'--json'],capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=40)
    value=json.loads(p.stdout)
    if not value.get('ok') and value.get('error',{}).get('code')=='window_not_focused' and '--restore-window' not in args:
        log('orca-focus-retry',name=name,error=value['error'])
        args=[*args,'--restore-window']
        p=subprocess.run(['orca','computer',*args,'--json'],capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=40)
        value=json.loads(p.stdout)
    stored=json.loads(json.dumps(value));snapshot=stored.get('result',{}).get('snapshot',{})
    if not tree_safe and 'treeText' in snapshot:snapshot['treeText']='Omitted before dedicated fixture filter to protect unrelated labels.'
    (OUT/(name+'.json')).write_text(json.dumps(stored,indent=2,ensure_ascii=False),encoding='utf-8')
    log('orca-command',name=name,args=args,exit=p.returncode,ok=value.get('ok'))
    assert value['ok'],value.get('error');return value
def selector(h):return ['--app',app,'--window-id',str(h)]
def windows():return orca('windows-'+str(time.time_ns()),['list-windows','--app',app],True)['result']['windows']
def state(h,name,query=None,screenshot=False):
    args=['get-app-state',*selector(h)]
    if query:args+=['--query',query]
    if not screenshot:args+=['--no-screenshot']
    return orca(name,args,query is not None or filtered.get(h,False))
def index(value,text,role):
    tree=value['result']['snapshot']['treeText']
    for line in tree.splitlines():
        match=re.match(r'\s*(\d+) '+re.escape(role)+' '+re.escape(text)+r'(?:,|$)',line)
        if match:return int(match[1])
    raise AssertionError('Control not found: '+role+' '+text)
def click_index(h,idx,name,safe=True):return orca(name,['click',*selector(h),'--element-index',str(idx),'--no-screenshot'],safe)
filtered={}
def open_overlay(name):
    # Expose only our fixture, then actual input checks this PID, not the DEV label.
    api('SetWindowPos',a['hwnd'],0,0,0,0,0,0x13)
    ownsel=['--app','pid:'+str(a['pid']),'--window-id',str(a['hwnd'])]
    orca(name+'-fixture-click',['click',*ownsel,'--x','80','--y','15','--no-screenshot'],True)
    orca(name+'-shortcut',['hotkey',*ownsel,'--key','Ctrl+Shift+4','--no-screenshot'],True)
    rows=windows();overlays=[x for x in rows if '영역 선택' in x['title']]
    if not overlays:
        log('shortcut-not-owned-or-unobserved',server=server)
        product(name+'-cli-fallback','app','select');rows=windows();overlays=[x for x in rows if '영역 선택' in x['title']]
    else:checks['actual_Ctrl_Shift_4']=True
    assert len(overlays)==1;return overlays[0]['id']
def choose_region(name,tiny=False):
    overlay=open_overlay(name)
    if tiny:
        orca(name+'-tiny-drag',['drag',*selector(overlay),'--from-x','300','--from-y','220','--to-x','310','--to-y','230','--no-screenshot'],True)
        rows=windows();checks['tiny_overlay_retained']=any(x['id']==overlay for x in rows)
        assert checks['tiny_overlay_retained']
        state(overlay,name+'-tiny-error',query='160')
    orca(name+'-drag',['drag',*selector(overlay),'--from-x','300','--from-y','220','--to-x','940','--to-y','640','--no-screenshot'],True)
    rows=windows();pickers=[x for x in rows if 'external-' in x['title']];assert len(pickers)==1
    h=pickers[0]['id'];query=state(h,name+'-search',query='검색');idx=index(query,'설치된 앱 / 열린 창 검색','편집')
    click_index(h,idx,name+'-search-click',False)
    orca(name+'-search-type',['type-text',*selector(h),'--text','ISSUE86-PRODUCT','--no-screenshot'],True)
    verified=state(h,name+'-verify-search',query='검색')
    if 'Value: ISSUE86-PRODUCT' not in verified['result']['snapshot']['treeText']:
        query=state(h,name+'-retry-search',query='검색');idx=index(query,'설치된 앱 / 열린 창 검색','편집')
        orca(name+'-search-set-value',['set-value',*selector(h),'--element-index',str(idx),'--value','ISSUE86-PRODUCT','--no-screenshot'],False)
        verified=state(h,name+'-verify-search-retry',query='검색')
    assert 'Value: ISSUE86-PRODUCT' in verified['result']['snapshot']['treeText'],'Search input delivery not confirmed'
    filtered_rows=state(h,name+'-verify-filter',query='전환 버튼')
    assert all('ISSUE86-PRODUCT' in line for line in filtered_rows['result']['snapshot']['treeText'].splitlines() if re.match(r'\s*\d+ 전환 버튼',line)),'Refuse screenshot with unrelated app/window labels'
    filtered[h]=True
    snapshot=state(h,name+'-filtered',screenshot=True)
    return h,snapshot
try:
    note=product('inspect-provenance','inspect','--id','issue86-product-6b023e9f');assert note['ok'] and note['instance']==info['id'] and note['note']['markdown']=='ISSUE86_PRODUCT_6b023e9f'
    log('provenance',source_sha=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),info=info,server=server,fixture=snap(a['hwnd']))
    original=snap(a['hwnd']);overlay=open_overlay('escape')
    orca('escape-cancel',['press-key',*selector(overlay),'--key','Escape','--no-screenshot'],True)
    checks['overlay_escape_cancel']=not [x for x in windows() if '영역 선택' in x['title']] and not product('escape-sessions','app','sessions')['sessions']
    h,snapshot=choose_region('welcome',tiny=True)
    sys.path.insert(0,str(root/'scripts'));from windows_uia import control_rect
    app_index=index(snapshot,'ISSUE86-PRODUCT-6b023e9f-UI','전환 버튼');click_index(h,app_index,'choose-app')
    button_rects={name:control_rect(h,name) for name in ('실행','새로 고침','취소')}
    log('actual-button-rects',rectangles=button_rects)
    checks['one_line_buttons']=all(r['height']<60 and r['width']>=78 for r in button_rects.values()) and len({r['y'] for r in button_rects.values()})==1
    checks['selection_does_not_run']=not product('selected-sessions','app','sessions')['sessions'] and snap(a['hwnd'])['rect']==original['rect']
    snapshot=state(h,'selected',screenshot=True);run_index=index(snapshot,'실행','단추');click_index(h,run_index,'run-existing')
    snapshot=state(h,'disambiguation',screenshot=True);tree=snapshot['result']['snapshot']['treeText']
    checks['existing_disambiguation']='이미 열린 창이 있습니다' in tree and 'ISSUE86-PRODUCT-UIA-' in tree and 'ISSUE86-PRODUCT-UIB-' in tree
    row=index(snapshot,'ISSUE86-PRODUCT-UIA-task6b023e9f','전환 버튼');click_index(h,row,'choose-window')
    snapshot=state(h,'window-selected',screenshot=True);click_index(h,index(snapshot,'실행','단추'),'run-window')
    time.sleep(.3);sessions=product('after-run','app','sessions')['sessions']
    checks['run_attaches_selected']=len(sessions)==1 and sessions[0]['window']['identity']['hwnd']==a['hwnd'] and snap(a['hwnd'])['topmost']
    orca('actual-attached-fixture',['get-app-state','--app','pid:'+str(a['pid']),'--window-id',str(a['hwnd'])],True)
    h,snapshot=choose_region('release')
    click_index(h,index(snapshot,'해제','단추'),'release-button')
    time.sleep(.3);snapshot=state(h,'released',screenshot=True);after=snap(a['hwnd'])
    checks['actual_welcome_release']=not product('released-sessions','app','sessions')['sessions'] and all(after[k]==original[k] for k in ('rect','topmost','visible','placement'))
    log('ui-restoration-observed',before=original,after=after)
    click_index(h,index(snapshot,'취소','단추'),'welcome-cancel')
    time.sleep(.2);checks['actual_welcome_cancel']=not [x for x in windows() if 'external-' in x['title']] and not product('cancel-sessions','app','sessions')['sessions']
except BaseException:log('failure',traceback=traceback.format_exc())
finally:
    (OUT/'summary.json').write_text(json.dumps(checks,indent=2),encoding='utf-8');print('EVIDENCE='+str(OUT),flush=True)
sys.exit(0 if len(checks)==9 and all(checks.values()) else 1)
