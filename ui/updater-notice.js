'use strict';

const noticeInvoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
const noticeEl = id => document.getElementById(id);
let noticeStatus = null;
let noticeBusy = false;
let readyRequestedVersion = null;
let noticeFeedbackVersion = null;
const noticeReview = window.gibbonUpdateReview(noticeEl('notice-review'), noticeInvoke, message => {
  noticeFeedbackVersion = noticeStatus?.version;
  noticeStatus = { ...noticeStatus, message };
  renderNotice(noticeStatus);
});
const labels = {
  ready: '새 버전이 준비됐습니다',
  preparing: '설치를 위한 안전 검사를 진행 중입니다',
  installing: '업데이트를 설치하고 다시 시작합니다',
  error: '업데이트를 처리하지 못했습니다',
};

function renderNotice(status) {
  noticeStatus = status || { phase: 'error', message: '' };
  noticeReview.render(noticeStatus.phase === 'ready' ? noticeStatus.review : null);
  noticeEl('notice-install').textContent = noticeReview.confirming ? '확인하고 재시작' : '업데이트하고 재시작';
  noticeEl('notice-dismiss').textContent = noticeReview.confirming ? '취소' : '나중에';
  const phase = Object.hasOwn(labels, noticeStatus.phase) ? noticeStatus.phase : 'error';
  noticeEl('notice-title').textContent = noticeReview.confirming ? '재시작 전에 확인하세요' : labels[phase];
  noticeEl('notice-version').hidden = noticeReview.confirming;
  noticeEl('notice-version').textContent = noticeStatus.version ? `새 버전 ${noticeStatus.version}` : '';
  const message = noticeEl('notice-message');
  message.textContent = noticeStatus.message || '업데이트 상태를 확인하지 못했습니다.';
  message.setAttribute('role', phase === 'error' ? 'alert' : 'status');
  message.hidden = phase !== 'error' && !(noticeFeedbackVersion && noticeFeedbackVersion === noticeStatus.version);
  noticeEl('notice-install').disabled = noticeBusy || phase !== 'ready';
  noticeEl('notice-dismiss').disabled = noticeBusy && phase === 'installing';
}

async function refreshNotice() {
  try {
    const status = await noticeInvoke('updater_notice_status');
    renderNotice(status);
    if (status.phase !== 'ready') readyRequestedVersion = null;
    else if (status.version && readyRequestedVersion !== status.version) {
      await noticeInvoke('updater_notice_ready');
      readyRequestedVersion = status.version;
    }
  } catch { renderNotice({ phase: 'error', message: '업데이트 상태를 확인하지 못했습니다. 설정에서 다시 확인하세요.' }); }
}

async function installNotice() {
  if (noticeBusy || noticeStatus?.phase !== 'ready') return;
  noticeBusy = true;
  renderNotice(noticeStatus);
  try {
    const result = await noticeReview.install('updater_notice_install');
    if (!result) return;
    noticeFeedbackVersion = result.phase === 'ready' ? result.version : null;
    renderNotice(result);
  }
  catch {
    noticeFeedbackVersion = noticeStatus.version;
    renderNotice({ ...noticeStatus, phase: 'ready', message: '안전 검사를 마치지 못했습니다. 다시 시도하세요.' });
  }
  finally { noticeBusy = false; renderNotice(noticeStatus); }
}

async function dismissNotice() {
  if (noticeReview.confirming) { noticeReview.cancel(); renderNotice(noticeStatus); return; }
  if (noticeBusy && noticeStatus?.phase === 'installing') return;
  noticeEl('notice-dismiss').disabled = true;
  try { await noticeInvoke('updater_notice_dismiss'); }
  catch {
    noticeEl('notice-message').hidden = false;
    noticeEl('notice-message').textContent = '안내를 닫지 못했습니다. 다시 시도하세요.';
    noticeEl('notice-message').setAttribute('role', 'alert');
    noticeEl('notice-dismiss').disabled = false;
  }
}

noticeEl('notice-install').onclick = installNotice;
noticeEl('notice-dismiss').onclick = dismissNotice;
refreshNotice();
setInterval(() => { if (!document.hidden) refreshNotice(); }, 1500);
