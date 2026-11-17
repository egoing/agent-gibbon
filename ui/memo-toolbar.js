// Build synchronously so each memo's action handlers can bind to the same nodes.
(() => {
  const controls = document.currentScript.parentElement;
  const kind = controls.dataset.kind;
  const actions = {
    text: ['font-size', 'copy', 'pin'],
    image: ['export', 'copy', 'pin'],
    terminal: ['font-size', 'pin'],
    web: ['font-size', 'copy', 'pin'],
  };
  const icons = {
    "toolbar-previous": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m14 6-6 6 6 6"/></svg>`,
    "font-size": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7v-2h13v2" />
  <path d="M10 5v14" />
  <path d="M12 19h-4" />
  <path d="M15 13v-1h6v1" />
  <path d="M18 12v7" />
  <path d="M17 19h2" /></svg>`,
    "save": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 4h13l3 3v13H4zM8 4v6h8V4M8 20v-6h8v6"/></svg>`,
    "export": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20l16 0" />
  <path d="M12 14l0 -10" />
  <path d="M12 14l4 -4" />
  <path d="M12 14l-4 -4" /></svg>`,
    "copy": `<svg class="copy-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7m0 2.667a2.667 2.667 0 0 1 2.667 -2.667h8.666a2.667 2.667 0 0 1 2.667 2.667v8.666a2.667 2.667 0 0 1 -2.667 2.667h-8.666a2.667 2.667 0 0 1 -2.667 -2.667z" />
  <path d="M4.012 16.737a2.005 2.005 0 0 1 -1.012 -1.737v-10c0 -1.1 .9 -2 2 -2h10c.75 0 1.158 .385 1.5 1" /></svg>`,
    "pin": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4l-8 4l8 4l8 -4l-8 -4" fill="currentColor" />
  <path d="M8 14l-4 2l8 4l8 -4l-4 -2" />
  <path d="M8 10l-4 2l8 4l8 -4l-4 -2" /></svg>`,
    "toolbar-next": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m10 6 6 6-6 6"/></svg>`,
    "toolbar-toggle": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4h-6a2 2 0 0 0 -2 2v12a2 2 0 0 0 2 2h8" />
  <path d="M18 20v-17" />
  <path d="M15 6l3 -3l3 3" /></svg>`,
    "close": `<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 6l-12 12" />
  <path d="M6 6l12 12" /></svg>`,
  };
  const labels = {
    'toolbar-previous': '이전 도구', 'toolbar-next': '다음 도구',
    'toolbar-toggle': '메모 접기', 'fold-toggle': '메모 접기',
    'font-size': kind === 'web' ? '페이지 확대/축소' : '글자 크기',
    save: '저장 · Command S / Ctrl S', export: '파일로 저장',
    copy: kind === 'web' ? '웹 주소 복사' : '본문 복사', pin: '상단 고정',
    close: {text: '메모 창 닫기', terminal: '터미널 종료', web: '웹 창 닫기'}[kind],
  };

  function button(id) {
    const el = document.createElement('button');
    el.type = 'button';
    el.id = id;
    el.setAttribute('aria-label', labels[id]);
    el.title = labels[id];
    el.innerHTML = icons[id === 'fold-toggle' ? 'toolbar-toggle' : id];
    if (id === 'toolbar-previous' || id === 'toolbar-next') {
      el.setAttribute('aria-controls', 'toolbar-items');
      el.hidden = true;
    }
    if (id === 'toolbar-toggle' || id === 'fold-toggle') el.setAttribute('aria-expanded', 'true');
    if (id === 'pin') el.setAttribute('aria-pressed', 'true');
    if (id === 'save') el.disabled = true;
    return el;
  }

  const title = document.createElement('span');
  title.id = 'chrome-title';
  if (kind !== 'text') {
    const name = document.createElement('span');
    name.id = 'title';
    name.textContent = kind === 'terminal' ? '터미널' : '웹';
    title.append(name);
  }
  const items = document.createElement('div');
  items.id = 'toolbar-items';
  // Captures and text share a WebView. Preserve listeners across kind changes.
  const ids = kind === 'text' ? ['fold-toggle', 'font-size', 'save', 'export', 'copy', 'pin'] : actions[kind];
  const buttons = ids.map(button);
  items.append(...buttons);
  controls.append(title, button('toolbar-previous'), items, button('toolbar-next'), button('toolbar-toggle'), button('close'));

  function setKind(value) {
    controls.dataset.kind = value;
    for (const el of buttons) el.hidden = !actions[value].includes(el.id);
  }
  window.gibbonMemoToolbar = {setKind};
  setKind(kind);
})();
