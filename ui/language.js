// Bind only application-owned labels. Never traverse editor contents, PTY
// output, file paths, note previews, URLs or the remote page webview.
(() => {
  const root = document.documentElement;
  root.setAttribute('data-language-pending', '');
  const catalog = window.jajanMessages;
  let state = { choice: 'system', locale: 'en', revision: -1 };
  const bindings = new WeakMap();
  function t(key, variables, locale = state.locale) {
    if (key && typeof key === 'object' && Object.hasOwn(key, 'message')) return t(key.message, key.values, locale);
    if (typeof key === 'string' && key.startsWith('Error: ') && Object.hasOwn(catalog, key.slice(7))) {
      key = key.slice(7);
    }
    if (typeof key === 'string' && !variables && key.includes(': ')) {
      const end = key.indexOf(': ') + 2;
      const template = key.slice(0, end) + '{detail}';
      if (Object.hasOwn(catalog, template)) return t(template, {detail: key.slice(end)}, locale);
    }
    const row = Object.hasOwn(catalog, key) ? catalog[key] : null;
    const template = locale === 'source' ? key : row?.[locale] ?? row?.en ?? key;
    if (!variables || typeof template !== 'string') return template;
    return template.replace(/\{(\w+)\}/g, (match, name) => typeof variables[name] === 'object' ? t(variables[name], undefined, locale) : variables[name] ?? match);
  }
  function write(el, field, record) {
    const value = t(record.key, record.variables);
    if (field === 'textContent') {
      // A later render may replace this node with user content or child controls.
      if (record.node.parentNode === el && record.node.data === record.last) {
        const text = record.prefix + value + record.suffix;
        if (record.node.data !== text) record.node.data = text;
        record.last = text;
      }
    } else if (el.getAttribute(field) === record.last) {
      if (value !== record.last) el.setAttribute(field, value);
      record.last = value;
    }
  }
  function remember(el, field, record) {
    let fields = bindings.get(el);
    if (!fields) {
      bindings.set(el, fields = new Map());
      el.setAttribute('data-language-bound', '');
    }
    if (record.node) record.last = record.node.data;
    fields.set(field, record);
    write(el, field, record);
  }
  function bind(el, field, key, variables) {
    if (field === 'textContent') {
      const value = t(key, variables);
      if (el.childNodes.length !== 1 || el.firstChild.nodeType !== Node.TEXT_NODE) {
        el.replaceChildren(document.createTextNode(value ?? ''));
      }
      remember(el, field, {key: key ?? '', variables, node: el.firstChild, prefix: '', suffix: ''});
    } else {
      remember(el, field, {key, variables, last: el.getAttribute(field)});
    }
  }
  function raw(el, field, value) {
    bindings.get(el)?.delete(field);
    if (field === 'textContent') el.textContent = value;
    else el.setAttribute(field, value);
  }
  function registerMarkup() {
    for (const el of document.querySelectorAll('[data-l10n]')) {
      for (const field of el.dataset.l10n.split(' ')) {
        if (bindings.get(el)?.has(field === 'text' ? 'textContent' : field)) continue;
        if (field !== 'text') {
          const key = el.getAttribute(field);
          if (Object.hasOwn(catalog, key)) bind(el, field, key);
          continue;
        }
        for (const node of el.childNodes) {
          if (node.nodeType !== Node.TEXT_NODE) continue;
          const key = node.data.trim();
          if (Object.hasOwn(catalog, key)) remember(el, 'textContent', {
            key, node, prefix: node.data.match(/^\s*/)[0], suffix: node.data.match(/\s*$/)[0],
          });
        }
      }
    }
  }
  function apply(snapshot) {
    if (!snapshot || snapshot.revision < state.revision) return;
    state = snapshot;
    root.lang = state.locale;
    for (const el of document.querySelectorAll('[data-language-bound]')) {
      for (const [field, record] of bindings.get(el) ?? []) write(el, field, record);
    }
    const select = document.getElementById('app-language');
    if (select) select.value = state.choice;
    window.dispatchEvent(new Event('jajan-language-change'));
  }
  window.jajanLanguage = { apply, bind, raw, t, source: key => t(key, undefined, 'source'), msg: (message, values) => ({message, values}), get choice() { return state.choice; }, get locale() { return state.locale; } };
  const initial = window.__TAURI__?.core.invoke('language_status')
    .then(apply).catch(error => console.error('Could not read language preference', error));
  document.addEventListener('DOMContentLoaded', async () => {
    registerMarkup();
    await initial;
    apply(state);
    root.removeAttribute('data-language-pending');
  }, { once: true });
})();
