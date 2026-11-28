'use strict';
(() => {
  const invoke = window.jajanInvoke || window.__TAURI__.core.invoke;
  const el = id => document.getElementById(id);
  let current, busy = false, clicks = 0, lastClick = -Infinity, revealed = false;
  const phases = { disabled: '사용 불가', idle: '대기', checking: '공식 피드 확인 중', downloading: '다운로드 중', ready: '준비 완료', preparing: '작업 보호 확인 중', installing: '설치·재시작 중', error: '오류' };
  function render(status) {
    const t = window.jajanLanguage.t;
    current = status;
    window.jajanLanguage.bind(el('settings-version'), 'textContent', `v${status.current_version || t('확인 불가')}`);
    const rows = [
      ['현재 버전', status.current_version || '확인 불가'],
      ['준비 버전', status.version || '없음'],
      ['시험 종류', status.roundtrip_active ? '호환 버전 왕복 시험' : status.manual_test ? (status.version === status.current_version ? '동일 버전 재설치 테스트' : '공식 배포판 설치 테스트') : '수동 시험 미준비'],
      ['다운로드', `${t(phases[status.phase] || '확인 불가')} · ${status.downloaded || 0}${status.total ? ` / ${status.total}` : ''} bytes`],
      ['서명·플랫폼·버전 검증', status.signature_verified ? '검증 완료' : '미검증'],
      ['캐시', status.cache_ready ? '저장 완료' : '준비 안 됨'],
      ['최근 준비 실패', ({manifest:'공식 피드·버전 확인', asset_url:'공식 플랫폼 파일 경로', download:'다운로드·서명 검증', signature_or_platform_binding:'서명·플랫폼·버전 바인딩', cache_write:'캐시 저장', ready:'준비 상태 반영'})[status.failure_stage] || '없음'],
      ['설치 정책', status.manual_test ? '수동 시험 대기 · 이 화면의 설치 버튼 필요' : '일반 자동 업데이트'],
    ];
    const list = el('developer-update-state');
    list.replaceChildren();
    for (const [label, value] of rows) {
      const dt = document.createElement('dt'), dd = document.createElement('dd');
      window.jajanLanguage.bind(dt, 'textContent', label); window.jajanLanguage.bind(dd, 'textContent', value); list.append(dt, dd);
    }
    window.jajanLanguage.bind(el('developer-update-message'), 'textContent', status.message || '');
    const working = ['checking', 'downloading', 'preparing', 'installing'].includes(status.phase);
    const pair = status.roundtrip_pair;
    const member = pair && [pair.previous, pair.latest].includes(status.current_version);
    if (el('developer-roundtrip-reason')) window.jajanLanguage.bind(el('developer-roundtrip-reason'), 'textContent', `${t(status.roundtrip_reason || '공식 호환 버전 쌍이 아직 없습니다.')}${pair ? ` ${pair.previous} ↔ ${pair.latest}` : ''}`);
    if (el('developer-previous')) el('developer-previous').disabled = busy || working || status.phase === 'disabled' || !member || status.current_version === pair.previous;
    if (el('developer-latest')) el('developer-latest').disabled = busy || working || status.phase === 'disabled' || !member || status.current_version === pair.latest;
    el('developer-prepare').disabled = busy || working || status.phase === 'disabled';
    el('developer-install').disabled = busy || status.phase !== 'ready' || !status.manual_test;
    window.jajanLanguage.bind(el('developer-prepare'), 'textContent', ['checking', 'downloading'].includes(status.phase) ? '최신 배포판 준비 중…' : '최신 배포판 준비');
    window.jajanLanguage.bind(el('developer-install'), 'textContent', status.phase === 'preparing' ? '작업 보호 확인 중…' : status.phase === 'installing' ? '설치·재시작 중…' : '준비한 버전 설치·재시작');
  }
  window.addEventListener('jajan-language-change', () => { if (current) render(current); });
  async function refresh() {
    try { render(await invoke('updater_status')); }
    catch { window.jajanLanguage.bind(el('developer-update-message'), 'textContent', '상태를 확인하지 못했습니다. 설정을 다시 열어 확인하세요.'); }
  }
  async function action(command, id, args) {
    if (busy || !current || el(id).disabled) return;
    busy = true; render(current);
    let failed = false;
    try { render(await invoke(command, args)); }
    catch {
      failed = true;
      await refresh();
    } finally {
      busy = false;
      if (current) render(current);
      if (failed) {
        window.jajanLanguage.bind(el('developer-update-message'), 'textContent', window.jajanLanguage.msg('작업을 완료하지 못했습니다. {reason}', {reason: window.jajanLanguage.msg(current?.message || '공식 피드·다운로드·서명·캐시 상태를 확인하고 다시 시도하세요.')}));
      }
    }
  }
  el('settings-version').onclick = () => {
    if (revealed) return;
    const now = performance.now();
    clicks = now - lastClick <= 600 ? clicks + 1 : 1;
    lastClick = now;
    if (clicks === 5) {
      revealed = true;
      el('settings-developer-nav').hidden = false;
    }
  };
  el('developer-prepare').onclick = () => action('updater_test_prepare', 'developer-prepare');
  el('developer-install').onclick = () => action('updater_test_install', 'developer-install');
  if (el('developer-previous')) el('developer-previous').onclick = () => action('updater_test_roundtrip', 'developer-previous', {destination:'previous'});
  if (el('developer-latest')) el('developer-latest').onclick = () => action('updater_test_roundtrip', 'developer-latest', {destination:'latest'});
  window.jajanDeveloperUpdater = { refresh };
  refresh();
  setInterval(() => {
    if (!document.hidden && !el('settings-page').hidden && !el('settings-developer').hidden) refresh();
  }, 2000);
})();
