// Memo display is independent of manager theme; OS is used only for system.
// This preference never changes stored memo colors or content.
export const memoDesignKey = 'agent-gibbon.memo-design';
const queries = new WeakMap();
export function readMemoAppearance(env = globalThis) {
  let design = 'gray';
  try {
    const value = env.localStorage?.getItem(memoDesignKey);
    if (['white', 'gray', 'black', 'system', 'color'].includes(value)) design = value;
  } catch {}
  if (design !== 'system') return {design};
  let systemDark = false;
  try {
    if (!queries.has(env)) queries.set(env, env.matchMedia?.('(prefers-color-scheme: dark)'));
    systemDark = queries.get(env)?.matches === true;
  } catch {}
  return {design, systemDark};
}
