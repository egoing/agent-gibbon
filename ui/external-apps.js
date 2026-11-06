'use strict';
const invoke=window.__TAURI__.core.invoke;
const win=window.__TAURI__.window.getCurrentWindow();
const el=id=>document.getElementById(id);
let apps=[],windows=[],selected=null,pending=null,busy=false,closed=false;
const request=(operation,options={})=>invoke('external_request',{request:{operation,app_id:null,window:null,session:null,region:null,...options}});
function message(error){el('status').textContent=String(error);}
function controls(){el('welcome').setAttribute('aria-busy',String(busy||Boolean(pending)));el('run-task').disabled=busy||Boolean(pending)||!selected;el('refresh').disabled=busy||Boolean(pending);}
function rows(){
 const query=el('search').value.toLocaleLowerCase();
 for(const [id,items,type] of [['apps',apps,'app'],['windows',windows,'window']]){
  el(id).replaceChildren();
  for(const item of items.filter(i=>(i.label||i.title).toLocaleLowerCase().includes(query))){
   const button=document.createElement('button');button.className='select-row';button.textContent=item.label||item.title;
   const key=type==='app'?item.id:item.window;button.setAttribute('aria-pressed',String(selected?.type===type&&selected.key===key));
   button.onclick=()=>{if(busy||pending)return;selected={type,key};rows();controls();};el(id).append(button);
  }
  if(!el(id).children.length){const empty=document.createElement('p');empty.className='hint';empty.textContent='표시할 항목이 없습니다.';el(id).append(empty);}
 }
 controls();
}
function sessions(data){
 el('sessions').replaceChildren();
 for(const item of data.sessions||[]){
  const row=document.createElement('div');row.className='session';
  const label=document.createElement('span');label.textContent=item.window.title+(item.alive?'':' (종료됨)');
  const release=document.createElement('button');release.textContent='해제';release.onclick=async()=>{
   release.disabled=true;try{await request('release',{session:item.session});message('원래 창 상태로 복원했습니다.');sessions(await request('sessions'));}catch(e){message(e);release.disabled=false;}
  };row.append(label,release);el('sessions').append(row);
 }
}
async function refresh(){
 if(busy||pending)return;busy=true;selected=null;controls();message('앱과 열린 창을 확인하고 있습니다…');
 // App discovery can fail independently; existing-window selection stays available.
 const [a,w,s]=await Promise.allSettled([request('list'),request('windows'),request('sessions')]);
 apps=a.status==='fulfilled'?a.value.apps:[];windows=w.status==='fulfilled'?w.value.windows:[];
 if(s.status==='fulfilled')sessions(s.value);
 el('limit').textContent=a.status==='fulfilled'?a.value.limit:'';
 const errors=[a,w,s].filter(r=>r.status==='rejected').map(r=>String(r.reason));message(errors.join('\n'));
 busy=false;rows();
}
async function handoff(){
 await invoke('external_complete');closed=true;await win.close();
}
async function run(){
 if(busy||pending||!selected)return;busy=true;controls();message('선택한 창을 준비하고 있습니다…');
 try{
  const result=selected.type==='app'?await request('open',{app_id:selected.key}):await request('attach',{window:selected.key});
  if(result.existing_windows){windows=result.existing_windows;selected=null;message('이미 열린 창이 있습니다. 이동할 창을 직접 선택한 뒤 실행하세요.');rows();}
  else if(result.launch){pending=result.launch;message('앱 시작 중… 취소해도 시작된 앱은 종료하지 않습니다.');poll();}
  else if(result.state==='attached'){message('상단 고정했습니다. 트레이에서 해제할 수 있습니다.');await handoff();}
 }catch(e){message(e);}
 finally{busy=false;controls();}
}
async function poll(){
 if(!pending||closed)return;
 try{
  const data=await request('sessions');const state=data.launches?.[pending];sessions(data);
  if(state?.state==='attached'){pending=null;message('상단 고정했습니다.');await handoff();return;}
  if(state?.state==='failed'||state?.state==='cancelled'){pending=null;message(state.error||'취소했습니다. 시작된 앱은 그대로 유지합니다.');controls();return;}
 }catch(e){pending=null;message(e);controls();return;}
 setTimeout(poll,300);
}
el('search').addEventListener('input',rows);
el('refresh').onclick=refresh;el('run-task').onclick=run;
el('cancel').onclick=async()=>{
 if(busy&&!pending)return;
 try{if(pending)await request('cancel',{session:pending});pending=null;closed=true;await win.close();}catch(e){message(e);}
};
(async()=>{try{const data=await invoke('external_welcome');el('region').textContent=`선택 영역: ${data.region[0]}, ${data.region[1]} · ${data.region[2]}×${data.region[3]} px`;await refresh();}catch(e){message(e);}})();
