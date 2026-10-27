export const durations = Object.freeze({ result: 2000, tip: 6000, error: 6000 });

// Editing owns the action slot; an unacknowledged revision survives behind it.
export function feedbackView({ editing, saving, acknowledging, highlightRevision, notice, sourcePath, color }) {
  const buttons = editing
    ? [{ id: 'cancel', label: '취소', disabled: saving }, { id: 'save', label: saving ? '저장 중' : '저장', disabled: saving, primary: true }]
    : highlightRevision !== null
      ? [{ id: 'acknowledge', label: '확인', disabled: acknowledging, primary: true }]
      : [];
  return {
    message: notice?.text || (editing ? (sourcePath ? '원본 파일에 저장' : '') : highlightRevision !== null ? '변경사항' : ''),
    revision: highlightRevision,
    error: notice?.kind === 'error', buttons, color: color || '#20252e',
  };
}
