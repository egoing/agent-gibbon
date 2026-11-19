(() => {
  const form=document.getElementById('web-form'),input=document.getElementById('web-url'),button=document.getElementById('run'),status=document.getElementById('status');
  installWebNavigationKeys(action => {
    if (!form.closest('main').hidden) return false;
    window.__TAURI__.core.invoke('web_chrome', {action}).catch(error => { status.textContent = String(error); });
  });
  let running=false;
  form.onsubmit=async event=>{
    event.preventDefault();if(running)return;
    running=true;button.disabled=true;form.setAttribute('aria-busy','true');status.textContent='웹페이지를 여는 중입니다.';
    try {const info=await window.__TAURI__.core.invoke('web_run',{url:input.value}); window.jajanWebChrome.apply(info);}
    catch(error){status.textContent=String(error);running=false;button.disabled=false;form.setAttribute('aria-busy','false');}
  };
})();
