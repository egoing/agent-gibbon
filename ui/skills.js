'use strict';
(() => {
  const invoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
  const el = id => document.getElementById(id);
  const rows = new Map();
  let state = [], busy = false, loaded = false;
  function feedback(message) {
    el('skills-feedback').textContent = message || '';
    el('skills-feedback').hidden = !message;
  }
  function render(snapshot) {
    if (snapshot) state = snapshot.skills;
    for (const skill of state) {
      let row = rows.get(skill.id);
      if (!row) {
        const details = document.createElement('details'); details.className = 'skill-item';
        const summary = document.createElement('summary');
        const dot = document.createElement('span'); dot.className = 'skill-dot'; dot.setAttribute('aria-hidden', 'true');
        const name = document.createElement('span'); name.className = 'skill-name'; name.textContent = skill.name;
        const status = document.createElement('span'); status.className = 'skill-status-label';
        summary.append(dot, name, status);
        const body = document.createElement('div'); body.className = 'skill-body';
        const description = document.createElement('p'); description.textContent = skill.description;
        const toggle = document.createElement('button'); toggle.type = 'button'; toggle.className = 'skill-switch'; toggle.setAttribute('role', 'switch'); toggle.setAttribute('aria-label', skill.name);
        const label = document.createElement('span');
        const track = document.createElement('span'); track.className = 'skill-switch-track'; track.setAttribute('aria-hidden', 'true');
        toggle.append(label, track);
        toggle.onclick = () => change(skill.id);
        body.append(description, toggle); details.append(summary, body);
        el('skills-list').append(details); row = { details, status, toggle, label }; rows.set(skill.id, row);
      }
      const value = skill.enabled ? 'On' : 'Off';
      row.details.dataset.enabled = String(skill.enabled);
      row.status.textContent = value;
      row.label.textContent = value;
      row.toggle.setAttribute('aria-checked', String(skill.enabled));
      row.toggle.setAttribute('aria-disabled', String(busy));
    }
    el('skills-loading').hidden = loaded;
    el('skills-list').setAttribute('aria-busy', String(busy));
  }
  async function refresh() {
    if (busy) return;
    busy = true; render();
    try {
      const snapshot = await invoke('skills_list');
      loaded = true; render(snapshot); feedback(snapshot.feedback);
    } catch (error) { loaded = true; feedback(String(error)); }
    finally { busy = false; render(); }
  }
  async function change(id) {
    const current = state.find(s => s.id === id);
    if (busy || !current) return;
    busy = true; render(); feedback('');
    try {
      const snapshot = await invoke('skills_set', { id, enabled: !current.enabled });
      render(snapshot); feedback(snapshot.feedback);
    } catch (error) {
      // Refresh authoritative state after an ambiguous transport/storage failure.
      // Keep the error visible even when the read succeeds.
      try { render(await invoke('skills_list')); } catch { /* Retain last known state. */ }
      feedback(String(error));
    } finally { busy = false; render(); }
  }
  window.gibbonSkills = { refresh };
})();
