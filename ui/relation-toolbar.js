(() => {
const items=document.getElementById('toolbar-items');
// Tabler Icons plug-connected, MIT: https://tabler.io/icons/icon/plug-connected
const connect = document.createElement('button');
connect.id='connect';connect.type='button';connect.hidden=true;
window.jajanLanguage.bind(connect, 'aria-label', '메모 연결');window.jajanLanguage.bind(connect, 'title', '메모 연결');
connect.innerHTML='<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 12l5 5l-1.5 1.5a3.536 3.536 0 1 1 -5 -5l1.5 -1.5"/><path d="M17 12l-5 -5l1.5 -1.5a3.536 3.536 0 1 1 5 5l-1.5 1.5"/><path d="M3 21l2.5 -2.5M18.5 5.5l2.5 -2.5M10 11l-2 2M13 14l-2 2"/></svg>';
items.append(connect);
let connecting=false;
window.__TAURI__?.core.invoke('relation_status').then(enabled=>{connect.hidden=!enabled;}).catch(()=>{});
connect.addEventListener('click',async()=>{
 connect.disabled=true;
 try{connecting=await window.__TAURI__.core.invoke('relation_begin');}
 catch(error){const status=document.getElementById('status');if(status)window.jajanLanguage.bind(status, 'textContent', String(error));}
 finally{connect.disabled=false;}
});
document.addEventListener('keydown',event=>{
 if(event.key==='Escape'&&connecting){event.preventDefault();event.stopImmediatePropagation();const generation=connecting;connecting=false;window.__TAURI__.core.invoke('relation_cancel',{generation}).catch(()=>{});}
},true);
})();
