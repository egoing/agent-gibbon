const el = id => document.getElementById(id);
const buttons = [el('access'), el('screen')];
const followUp = el('follow-up');
let visit = null;
let timer;
let inFlight = false;
let queuedOpen = null;

function setScope(shortcut) {
  el('screen').closest('section').hidden = shortcut;
}
setScope(new URLSearchParams(location.search).has('shortcut'));
window.setPermissionScope = value => { setScope(value); refresh(); };

function showHint(id, message, paths) {
  const card = el(id).closest('section');
  if (followUp.parentElement !== card) card.append(followUp);
  window.jajanLanguage.bind(el('hint'), 'textContent', message);
  followUp.querySelector('details').hidden = !paths;
  followUp.hidden = false;
}

async function check(openId) {
  clearTimeout(timer);
  if (inFlight) {
    if (openId) queuedOpen = openId;
    return;
  }
  inFlight = true;
  if (openId) {
    visit = { id: openId, left: false, returned: false };
    followUp.hidden = true;
    followUp.querySelector('details').open = false;
  }

  try {
    const action = openId || (visit ? 'observe-' + visit.id : 'status');
    const [screen, access, path, , application] = await window.__TAURI__.core.invoke('permission_guide', { action });
    const allowed = { screen, access };
    for (const button of buttons) {
      window.jajanLanguage.bind(button, 'textContent', allowed[button.id] ? '확인됨 ✓' : '설정 열기');
      button.disabled = allowed[button.id];
    }
    if (visit && allowed[visit.id]) {
      visit = null;
      followUp.hidden = true;
    } else if (visit?.returned) {
      if (el('path').textContent !== path) el('path').textContent = path;
      el('application-label').hidden = !application;
      if (el('application-path').textContent !== (application || '')) {
        el('application-path').textContent = application || '';
      }
      el('application-path').hidden = !application;
      showHint(visit.id, '아직 확인되지 않았습니다. 설정에서 실행 중인 앱이 허용되어 있는지 확인하세요.', true);
    } else if (!visit) {
      followUp.hidden = true;
    }
  } catch {
    if (openId) {
      visit = null;
      showHint(openId, '설정을 열지 못했습니다. 다시 시도하세요.', false);
    } else if (visit?.returned) {
      showHint(visit.id, '권한 상태를 확인하지 못했습니다. 잠시 후 다시 확인합니다.', false);
    } else if (!visit) {
      for (const button of buttons) {
        if (button.disabled) {
          window.jajanLanguage.bind(button, 'textContent', '확인 중');
        }
      }
    }
  } finally {
    inFlight = false;
    if (queuedOpen) {
      const next = queuedOpen;
      queuedOpen = null;
      check(next);
    } else {
      timer = setTimeout(refresh, 1000);
    }
  }
}

function refresh() { check(null); }
function leave() {
  if (visit) visit.left = true;
}
function resume() {
  if (visit?.left) visit.returned = true;
  refresh();
}
for (const button of buttons) button.onclick = () => check(button.id);
window.addEventListener('blur', leave);
window.addEventListener('focus', resume);
document.addEventListener('visibilitychange', () => document.hidden ? leave() : resume());
window.addEventListener('pagehide', () => clearTimeout(timer));
refresh();
