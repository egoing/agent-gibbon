(() => {
 const {invoke}=window.__TAURI__.core;
 const svg=document.querySelector('svg'),line=document.getElementById('line'),box=document.getElementById('target'),hint=document.getElementById('hint');
 const marker=document.getElementById('arrow');
 let ready=false,latest=null,pending=false,frame=0,choosing=false;
 const cancel=()=>invoke('relation_cancel').catch(error=>{window.jajanLanguage.bind(hint, 'textContent', String(error));});
 window.__TAURI__.event.listen('relation-preview',({payload:p})=>{
  svg.setAttribute('viewBox',`0 0 ${p.width} ${p.height}`);
  line.style.strokeWidth=2*p.scale;box.style.strokeWidth=2*p.scale;
  line.setAttribute('d',p.points?window.relationCurve(p.points,'end',p.scale,marker).d:'');
  box.style.display=p.highlight?'block':'none';
  if(p.highlight)for(const [k,v] of Object.entries(p.highlight))box.setAttribute(k,v);
  window.jajanLanguage.bind(hint, 'textContent', p.target?'클릭하면 이 메모와 연결합니다.':'연결할 메모 위로 이동한 뒤 클릭하세요. 겹친 메모는 먼저 떨어뜨려 주세요.');
 }).then(()=>invoke('relation_picker_info')).then(()=>{ready=true;}).catch(error=>{window.jajanLanguage.bind(hint, 'textContent', String(error));});
 async function move(){
  frame=0;if(!ready||pending||choosing||!latest)return;
  const p=latest;latest=null;pending=true;
  try{await invoke('relation_pick',{...p,choose:false});}catch(error){window.jajanLanguage.bind(hint, 'textContent', String(error));}finally{pending=false;if(latest&&!choosing)frame=requestAnimationFrame(move);}
 }
 document.addEventListener('pointermove',event=>{latest={x:event.clientX,y:event.clientY};if(!frame&&!pending)frame=requestAnimationFrame(move);});
 document.addEventListener('click',async event=>{
  if(event.target.closest('aside')||!ready||choosing)return;
  choosing=true;
  try{const ok=await invoke('relation_pick',{x:event.clientX,y:event.clientY,choose:true});if(!ok){window.jajanLanguage.bind(hint, 'textContent', '겹치지 않은 다른 메모를 클릭해 주세요.');choosing=false;}}
  catch(error){window.jajanLanguage.bind(hint, 'textContent', String(error));choosing=false;}
 });
 document.getElementById('cancel').addEventListener('click',cancel);
 document.addEventListener('keydown',event=>{if(event.key==='Escape'){event.preventDefault();cancel();}});
})();
