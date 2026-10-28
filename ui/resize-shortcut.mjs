export function wheelRequest(event) {
  if (event.shiftKey || [event.metaKey, event.ctrlKey, event.altKey].filter(Boolean).length !== 1) return null;
  if (!Number.isFinite(event.deltaY) || event.deltaY === 0) return null;
  const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 240 : 1;
  return { axis: event.metaKey ? 'both' : event.ctrlKey ? 'horizontal' : 'vertical', delta: event.deltaY * unit };
}
export function resizeTip(isMac) {
  return isMac ? '휠 + ⌘ 비율 · Ctrl 좌우 · Option 상하' : '휠 + Win 비율 · Ctrl 좌우 · Alt 상하';
}
