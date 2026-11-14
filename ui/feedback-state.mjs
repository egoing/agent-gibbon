export const durations = Object.freeze({ result: 2000, tip: 6000, error: 6000 });

// Editing owns the action slot; an unacknowledged revision survives behind it.
export function feedbackView({ editing, saving, exporting, highlightRevision, notice, color }) {
  const buttons = editing
    ? [{ id: 'cancel', label: '취소', disabled: saving || exporting }, { id: 'save', label: saving ? '저장 중' : '저장', disabled: saving || exporting, primary: true }]
    : [];
  return {
    message: notice?.text || '',
    revision: highlightRevision,
    error: notice?.kind === 'error', buttons, color: color || '#20252e',
  };
}
