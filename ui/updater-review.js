'use strict';

// Review state belongs to this window. Polling must never confirm a changed set of tasks.
window.gibbonUpdateReview = function updateReview(container, invoke, reportError) {
  let review = null;
  let confirming = false;
  let key = '';
  const reason = target => {
    if (target.kind === 'editor') return '편집 중 · 저장하거나 취소하세요';
    if (target.kind === 'web') return '웹 창 열림';
    return target.pid ? '터미널 실행 중' : '터미널 창 열림';
  };
  function paint() {
    container.replaceChildren();
    container.hidden = !review?.targets.length;
    if (container.hidden) return;
    for (const target of confirming ? [] : review.targets) {
      const row = document.createElement('div');
      const name = document.createElement('span');
      name.textContent = target.title + ' · ' + reason(target);
      const show = document.createElement('button');
      show.type = 'button';
      show.textContent = '보기';
      show.setAttribute('aria-label', target.title + ' 보기');
      show.onclick = () => invoke('updater_show_target', { label: target.label })
        .catch(() => reportError('창을 열 수 없습니다. 작업 상태를 다시 확인하세요.'));
      row.append(name, show);
      container.append(row);
    }
    if (confirming) {
      const message = document.createElement('p');
      const lines = [];
      if (review.targets.some(t => t.kind === 'web')) lines.push('웹 창은 같은 주소로 엽니다.');
      if (review.targets.some(t => t.kind === 'terminal')) {
        if (review.targets.some(t => t.pid)) lines.push('실행 중인 터미널 프로세스는 종료됩니다.');
        lines.push('터미널 창은 복원하고 명령은 실행하지 않습니다.');
      }
      message.textContent = lines.join(' ');
      container.append(message);
      const scroller = container.closest?.('.notice-content');
      if (scroller) scroller.scrollTop = 0;
    }
  }
  return {
    render(next) {
      const newKey = JSON.stringify(next || null);
      if (key === newKey) return;
      key = newKey;
      review = next;
      confirming = false;
      paint();
    },
    get confirming() { return confirming; },
    cancel() { confirming = false; paint(); },
    async install(command) {
      if (!review?.targets.length) return invoke(command);
      if (review.targets.some(t => t.kind === 'editor')) {
        reportError('편집 중인 메모를 저장하거나 취소한 뒤 다시 시도하세요.');
        return null;
      }
      if (!review.restores_sessions) {
        reportError('이 버전은 열린 작업을 복원할 수 없습니다. 웹·터미널 창을 닫은 뒤 다시 시도하세요.');
        return null;
      }
      if (!confirming) { confirming = true; paint(); return null; }
      const confirmed = review;
      confirming = false;
      paint();
      return invoke(command, { confirmed });
    },
  };
};
