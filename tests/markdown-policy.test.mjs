import test from 'node:test';
import assert from 'node:assert/strict';
import { preservesMarkdown } from '../ui/markdown-policy.mjs';
test('format-only normalization is allowed without treating it as a no-op source write',()=>{
  assert.equal(preservesMarkdown('タイトル\n===\n\n* **한글**\r\n','# タイトル\n\n- **한글**\n'),true);
  assert.equal(preservesMarkdown('- [ ] 대기\n- [x] 완료','* [ ] 대기\n* [x] 완료\n'),true);
});
test('semantic source loss in HTML, references, code meta, tables, tasks is rejected',()=>{
  for(const [source,next] of [
    ['<!--보존-->\n본문','본문'],['[x][a]\n\n[a]: https://example.com','[x](https://example.com)'],
    ['```js title\n코드\n```','```js\n코드\n```'],['- [x] 완료','- [ ] 완료'],
    ['|a|b|\n|-|-|\n|1|2|','a b 1 2'],['<b>문자</b>','**문자**'],
  ])assert.equal(preservesMarkdown(source,next),false,source);
});
