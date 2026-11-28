(() => {
 const {invoke}=window.__TAURI__.core;
 const form=document.getElementById('settings'),label=document.getElementById('label'),arrow=document.getElementById('arrow'),error=document.getElementById('error'),save=document.getElementById('save');
 let saving=false,generation=null;
 invoke('relation_editor_state').then(state=>{const from=state.fromTitle??state.from,to=state.toTitle??state.to;generation=state.generation;document.getElementById('pair').textContent=`${from} → ${to}`;label.focus();}).catch(e=>{window.jajanLanguage.bind(error, 'textContent', String(e));save.disabled=true;});
 const cancel=()=>{if(!saving)invoke('relation_cancel',{generation}).catch(e=>{window.jajanLanguage.bind(error, 'textContent', String(e));});};
 document.getElementById('cancel').addEventListener('click',cancel);
 document.addEventListener('keydown',event=>{if(event.key==='Escape'){event.preventDefault();cancel();}});
 form.addEventListener('submit',async event=>{
  event.preventDefault();if(saving)return;saving=true;save.disabled=true;window.jajanLanguage.bind(error, 'textContent', '연결을 저장하고 있습니다…');
  try{await invoke('relation_save',{label:label.value,arrow:arrow.value,generation});}catch(e){window.jajanLanguage.bind(error, 'textContent', String(e));saving=false;save.disabled=false;}
 });
})();
