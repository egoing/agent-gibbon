export const durations = Object.freeze({ result: 2000, tip: 6000, error: 6000 });

// Editing owns the action slot; an unacknowledged revision survives behind it.
export function feedbackView({ editing, saving, exporting, highlightRevision, notice, color, composition = false, sourceProtected = false, unified = false, autosaveBlocked = false }) {
  const buttons = unified
    ? autosaveBlocked && editing ? [{ id: 'cancel', label: '변경 취소', disabled: saving || composition }, { id: 'save', label: '다시 저장', disabled: saving || composition || sourceProtected, primary: true }] : []
    : editing
    ? [{ id: 'cancel', label: '취소', disabled: saving || exporting || composition }, { id: 'save', label: saving ? '저장 중' : '저장', disabled: saving || exporting || composition || sourceProtected, primary: true }]
    : [];
  return {
    message: notice?.text || (unified && saving ? '자동 저장 중…' : unified && editing ? composition ? '입력 중…' : '자동 저장 대기…' : ''),
    revision: highlightRevision,
    error: notice?.kind === 'error', buttons, color: color || '#20252e',
  };
}
