'use strict';

const updaterInvoke = window.jajanInvoke || window.__TAURI__.core.invoke;
const updaterEl = id => document.getElementById(id);
const phaseText = {
  disabled: '자동 업데이트를 사용할 수 없습니다.',
  idle: '업데이트를 확인할 수 있습니다.',
  checking: '업데이트를 확인하는 중입니다.',
  downloading: '업데이트를 다운로드하는 중입니다.',
  ready: '업데이트를 설치할 준비가 되었습니다.',
  preparing: '설치를 준비하는 중입니다.',
  installing: '업데이트를 설치하는 중입니다.',
  error: '업데이트를 처리하지 못했습니다.',
};
let updaterStatus = null;
let updaterBusy = false;

function renderUpdater(status) {
  updaterStatus = status || { phase: 'error', message: '' };
  const phase = Object.hasOwn(phaseText, updaterStatus.phase) ? updaterStatus.phase : 'error';
  const version = updaterStatus.version || null;
  updaterEl('updater-status').textContent = phaseText[phase];
  updaterEl('updater-version').textContent = version
    ? `현재 버전 ${updaterStatus.current_version || ''} · 새 버전 ${version}`
    : `현재 버전 ${updaterStatus.current_version || '확인할 수 없음'}`;
  const progress = updaterEl('updater-progress');
  const hasProgress = phase === 'downloading' && Number.isFinite(updaterStatus.total) && updaterStatus.total > 0;
  progress.hidden = !hasProgress;
  if (hasProgress) {
    progress.max = updaterStatus.total;
    progress.value = Math.min(Math.max(Number(updaterStatus.downloaded) || 0, 0), updaterStatus.total);
  }
  const message = updaterEl('updater-message');
  message.textContent = updaterStatus.message || (phase === 'disabled' ? '개발용 빌드에서는 자동 업데이트가 꺼져 있습니다.' : '');
  message.setAttribute('role', phase === 'error' ? 'alert' : 'status');
  message.hidden = !message.textContent;
  updaterEl('updater-check').disabled = updaterBusy || phase === 'disabled' || ['checking', 'downloading', 'preparing', 'installing'].includes(phase);
  updaterEl('updater-check').textContent = phase === 'error' ? '다시 확인' : '지금 확인';
  updaterEl('updater-install').textContent = phase === 'preparing' ? '작업 상태 확인 중…' : phase === 'installing' ? '설치하고 재시작 중…' : '업데이트하고 재시작';
  updaterEl('updater-install').disabled = updaterBusy || phase !== 'ready';
  updaterEl('updater-install').hidden = !['ready', 'preparing', 'installing'].includes(phase);
}

async function refreshUpdater() {
  try { renderUpdater(await updaterInvoke('updater_status')); }
  catch { renderUpdater({ phase: 'error', current_version: updaterStatus?.current_version, message: '업데이트 상태를 확인하지 못했습니다. 다시 시도하세요.' }); }
}

async function runUpdaterAction(command) {
  if (updaterBusy || (command === 'updater_install' && updaterStatus?.phase !== 'ready')) return;
  updaterBusy = true;
  renderUpdater(updaterStatus);
  try { renderUpdater(await updaterInvoke(command)); }
  catch {
    const message = command === 'updater_check'
      ? '업데이트를 확인하지 못했습니다. 다시 시도하세요.'
      : '업데이트 설치를 시작하지 못했습니다. 다시 시도하세요.';
    renderUpdater({ phase: 'error', current_version: updaterStatus?.current_version, message });
  } finally {
    updaterBusy = false;
    renderUpdater(updaterStatus);
  }
}

updaterEl('updater-check').onclick = () => runUpdaterAction('updater_check');
updaterEl('updater-install').onclick = () => runUpdaterAction('updater_install');
window.jajanUpdater = { refresh: refreshUpdater };

setInterval(() => {
  if (!document.hidden && !updaterEl('settings-page').hidden && !updaterEl('settings-updater').hidden) refreshUpdater();
}, 2000);
