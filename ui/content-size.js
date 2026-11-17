// The second step preserves the original size of each content surface.
window.gibbonContentSize = (() => {
  const steps = [
    { text: 10, terminal: 11, web: 0.85 },
    { text: 12, terminal: 13, web: 1 },
    { text: 14, terminal: 14, web: 1.15 },
    { text: 16, terminal: 16, web: 1.30 },
    { text: 18, terminal: 18, web: 1.50 },
  ];
  const initial = 1;
  function bind(kind, button, apply, report) {
    let index = initial;
    let pending = false;
    const label = () => {
      const value = steps[index][kind];
      const size = kind === 'web' ? `페이지 배율 ${Math.round(value * 100)}%` : `글자 크기 ${value}px`;
      button.title = `${size} · 클릭하여 변경`;
      button.setAttribute('aria-label', button.title);
    };
    label();
    button.onclick = async () => {
      if (pending || button.disabled) return;
      pending = true;
      try {
        const next = (index + 1) % steps.length;
        await apply(steps[next][kind]);
        index = next;
        label();
      } catch (error) {
        report(error);
      } finally {
        pending = false;
      }
    };
  }
  return { bind, defaultValue: kind => steps[initial][kind] };
})();
