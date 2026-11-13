export const durations = Object.freeze({ result: 2000, tip: 6000, error: 6000 });

// Editing owns the action slot; an unacknowledged revision survives behind it.
export function feedbackView({ editing, saving, exporting, highlightRevision, notice, sourcePath, color }) {
  const buttons = editing
    ? [{ id: 'export', label: '파일 내보내기', disabled: saving || exporting }, { id: 'cancel', label: '취소', disabled: saving || exporting }, { id: 'save', label: saving ? '반영 중' : '메모에 반영', disabled: saving || exporting, primary: true }]
    : [];
  return {
    message: notice?.text || (editing && sourcePath ? '원본 파일에 저장' : ''),
    revision: highlightRevision,
    error: notice?.kind === 'error', buttons, color: color || '#20252e',
  };
}
