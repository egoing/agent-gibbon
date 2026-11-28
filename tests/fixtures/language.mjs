// Most component fixtures deliberately omit the page bootstrap and DOM parser.
// Keep their original source-language assertions and minimal element doubles.
export function sourceLanguageWindow(context) {
  const win = context.window;
  function text(key, values) {
    if (key && typeof key === 'object') return text(key.message, key.values);
    if (!values) return key;
    return key.replace(/\{(\w+)\}/g, (_, name) => text(values[name]));
  }
  win.jajanLanguage ??= {
    t: text, source: text, msg: (message, values) => ({message, values}),
    raw(el, field, value) { if (field === 'textContent') el.textContent = value; else el.setAttribute(field, value); },
    bind(el, field, key, values) {
      const value = text(key, values);
      if (field === 'textContent' || field === 'title') el[field] = value;
      else el.setAttribute(field, value);
    },
  };
  win.addEventListener ??= () => {};
  return context;
}
