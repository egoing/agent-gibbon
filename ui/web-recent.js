(() => {
  const section = document.getElementById('web-recent');
  const list = document.getElementById('web-recent-list');
  const form = document.getElementById('web-form');
  const input = document.getElementById('web-url');
  const status = document.getElementById('status');
  const invoke = window.__TAURI__.core.invoke;
  let generation = 0, deleting = false;
  const busy = () => form.getAttribute('aria-busy') === 'true';
  const render = urls => {
    list.replaceChildren();
    section.hidden = urls.length === 0;
    urls.forEach(url => {
      const row = document.createElement('li');
      const open = document.createElement('button');
      open.type = 'button'; open.className = 'recent-open';
      open.textContent = url; open.title = url;
      window.jajanLanguage.bind(open, 'aria-label', window.jajanLanguage.msg('{url}로 이동', {url}));
      open.onclick = () => {
        if (busy() || deleting) return;
        input.value = url;
        form.requestSubmit();
      };
      const remove = document.createElement('button');
      remove.type = 'button'; remove.className = 'recent-remove';
      window.jajanLanguage.bind(remove, 'textContent', '삭제');
      window.jajanLanguage.bind(remove, 'aria-label', window.jajanLanguage.msg('{url} 최근 주소 삭제', {url}));
      remove.onclick = async () => {
        if (busy() || deleting) return;
        deleting = true; ++generation;
        const index = [...list.children].indexOf(row);
        const restoreFocus = document.activeElement === remove;
        let focusAfter = remove;
        for (const button of list.querySelectorAll('button')) button.disabled = true;
        try {
          const urls = await invoke('web_recent', {remove: url});
          render(urls);
          focusAfter = list.children[Math.min(index, urls.length - 1)]?.querySelector('.recent-remove') || input;
          window.jajanLanguage.bind(status, 'textContent', '최근 주소를 삭제했습니다.');
        } catch (error) {
          window.jajanLanguage.bind(status, 'textContent', String(error));
        } finally {
          deleting = false;
          for (const button of list.querySelectorAll('button')) button.disabled = busy();
          if (restoreFocus) focusAfter.focus();
        }
      };
      open.disabled = remove.disabled = busy() || deleting;
      row.append(open, remove); list.append(row);
    });
  };
  const refresh = async () => {
    if (deleting || busy()) return;
    const ticket = ++generation;
    try {
      const urls = await invoke('web_recent');
      if (ticket === generation) render(urls);
    } catch (error) {
      if (ticket === generation) window.jajanLanguage.bind(status, 'textContent', String(error));
    }
  };
  new MutationObserver(() => {
    for (const button of list.querySelectorAll('button')) button.disabled = busy() || deleting;
  }).observe(form, {attributes: true, attributeFilter: ['aria-busy']});
  window.addEventListener('focus', refresh);
  if (window.jajanRegionFrame) window.jajanRegionFrame.prepare(refresh);
  else refresh();
})();
