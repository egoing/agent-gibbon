const el = id => document.getElementById(id);
const buttons = ['screen', 'access', 'check'].map(el);
let pending = null;
let timer;
let busy = false;

async function check(action) {
  if (busy) return;
  busy = true;
  clearTimeout(timer);
  const opening = action === 'screen' || action === 'access';
  if (opening) {
    pending = action;
    document.querySelector('aside').hidden = false;
    el('pane').textContent = '시스템 설정 → 개인정보 보호 및 보안 → ' +
      (action === 'screen' ? '화면 및 시스템 오디오 녹음' : '손쉬운 사용');
  }
  try {
    const [screen, access, path] = await window.__TAURI__.core.invoke('permission_guide', { action });
    el('path').textContent = path;
    for (const [id, allowed] of [['screen', screen], ['access', access]]) {
      el(id).textContent = allowed ? '확인됨 ✓' : '허용';
      el(id).disabled = allowed;
    }
    const confirmed = pending && (pending === 'screen' ? screen : access);
    const message = confirmed ? '선택한 권한을 확인했습니다.' :
      pending ? '시스템 설정에서 현재 실행 앱의 스위치를 켜세요. 이미 켰다면 macOS의 재실행 안내를 확인하세요.' :
      screen ? '화면 캡처 접근을 확인했습니다.' : '현재 앱의 화면 캡처 접근을 확인하지 못했습니다.';
    if (el('message').textContent !== message) el('message').textContent = message;
    if (confirmed && !opening) pending = null;
  } catch {
    el('message').textContent = '확인하지 못했습니다. 설정을 확인한 뒤 다시 시도하세요.';
  } finally {
    busy = false;
    if (pending) timer = setTimeout(refresh, 1000);
  }
}
function refresh() {
  return check(pending ? 'observe-' + pending : 'status');
}
buttons.forEach(button => {
  button.onclick = () => button.id === 'check' ? refresh() : check(button.id);
});
window.addEventListener('focus', refresh);
refresh();
