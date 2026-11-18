const el = id => document.getElementById(id);
const buttons = ['screen', 'access', 'check'].map(el);
let pending = null;
let timer;
let busy = false;
let shortcut = new URLSearchParams(location.search).has('shortcut');
window.setPermissionScope = value => {
  shortcut = value;
  el('screen').closest('section').hidden = shortcut;
  if (!busy) refresh();
};
el('screen').closest('section').hidden = shortcut;

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
    const [screen, access, path, inputReady, application] = await window.__TAURI__.core.invoke('permission_guide', { action });
    el('path').textContent = path;
    el('application-label').hidden = !application;
    el('application-path').hidden = !application;
    el('application-path').textContent = application || '';
    el('selection-help').textContent = application ?
      '목록의 앱을 확인하세요. 추가가 필요하면 + 버튼을 누른 뒤 아래 .app 번들을 선택하세요. 앱 업데이트 후 같은 경로가 이미 켜져 있어도 확인되지 않으면 해당 앱 항목을 제거한 뒤 현재 앱을 다시 추가하고 허용하세요.' :
      '현재는 .app 번들 밖에서 실행 중입니다. 실행한 터미널이나 개발 도구의 권한을 확인하세요. 진단용 실행 파일 경로는 아래에서 확인할 수 있습니다.';
    for (const [id, allowed] of [['screen', screen], ['access', access]]) {
      el(id).textContent = allowed ? '확인됨 ✓' : '설정 열기';
      el(id).disabled = allowed;
    }
    const confirmed = pending && (pending === 'screen' ? screen : access);
    const message = confirmed ? '선택한 권한을 확인했습니다.' :
      pending ? 'macOS에서 현재 실행 앱의 권한이 아직 확인되지 않습니다. 설정이 켜져 있다면 아래 앱 경로와 등록된 앱이 일치하는지 확인하세요.' :
      shortcut ? (access ? (inputReady ? '손쉬운 사용 권한과 단축키 입력 감시를 확인했습니다.' : '손쉬운 사용 권한을 확인했습니다. 입력 감시를 연결 중입니다. 계속 작동하지 않으면 다른 Gibbon 실행 여부를 확인하세요.') :
        '보조키 조합 단축키에 손쉬운 사용 권한이 필요합니다. 다른 앱에서 입력이 감지되지 않으면 Gibbon 창을 선택하거나 트레이의 권한 설정을 여세요.') :
      !access ? 'macOS에서 현재 실행 앱의 손쉬운 사용 권한이 확인되지 않습니다. 이미 허용했다면 설정에 등록된 앱과 아래 앱 경로를 확인하세요.' :
      screen ? '손쉬운 사용과 화면 캡처 접근을 확인했습니다.' : '손쉬운 사용은 확인됐으며 화면 캡처 접근은 아직 확인되지 않습니다.';
    if (el('message').textContent !== message) el('message').textContent = message;
    if (confirmed && !opening) pending = null;
  } catch {
    el('message').textContent = '확인하지 못했습니다. 설정을 확인한 뒤 다시 시도하세요.';
  } finally {
    busy = false;
    timer = setTimeout(refresh, 1000);
  }
}
function refresh() {
  return check(pending ? 'observe-' + pending : 'status');
}
buttons.forEach(button => {
  button.onclick = () => button.id === 'check' ? refresh() : check(button.id);
});
window.addEventListener('focus', refresh);
document.addEventListener('visibilitychange', () => {
  if (!document.hidden) refresh();
});
window.addEventListener('pagehide', () => clearTimeout(timer));
refresh();
