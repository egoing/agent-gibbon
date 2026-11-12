(() => {
  const form=document.getElementById('web-form'),input=document.getElementById('web-url'),button=document.getElementById('run'),status=document.getElementById('status');
  let running=false;
  form.onsubmit=async event=>{
    event.preventDefault();if(running)return;
    running=true;button.disabled=true;form.setAttribute('aria-busy','true');status.textContent='웹페이지를 여는 중입니다.';
    try {await window.__TAURI__.core.invoke('web_run',{url:input.value}); document.querySelector('main').hidden=true; document.getElementById('title').textContent=input.value;}
    catch(error){status.textContent=String(error);running=false;button.disabled=false;form.setAttribute('aria-busy','false');}
  };
})();
