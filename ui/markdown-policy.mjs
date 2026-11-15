import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkGfm from 'remark-gfm';

const parser = unified().use(remarkParse).use(remarkGfm);
// Formatting choices may change, but never silently discard source constructs.
function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value)
      .filter(([key]) => !['position', 'spread'].includes(key))
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([key, item]) => [key, canonical(item)]));
  }
  return value;
}
export function preservesMarkdown(source, serialized) {
  return JSON.stringify(canonical(parser.parse(source))) === JSON.stringify(canonical(parser.parse(serialized)));
}
