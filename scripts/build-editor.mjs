import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
const root = fileURLToPath(new URL('..', import.meta.url));
const result = await build({
  absWorkingDir: root, entryPoints: ['ui/milkdown-editor.js'], outfile: 'ui/generated/milkdown.js',
  bundle: true, format: 'esm', platform: 'browser', target: 'es2020',
  minify: true, legalComments: 'external', charset: 'utf8', metafile: true,
});
// Ship the actual bundled packages' notices, including transitive dependencies.
const packages = new Map();
for (const input of Object.keys(result.metafile.inputs).filter(name => name.includes('node_modules/'))) {
  let dir = dirname(resolve(root, input));
  while (dir !== dirname(dir)) {
    const manifest = join(dir, 'package.json');
    if (existsSync(manifest)) {
      const pkg = JSON.parse(readFileSync(manifest, 'utf8'));
      if (pkg.name) {
        const license = ['LICENSE', 'LICENSE.md', 'LICENSE.txt', 'license', 'license.md'].map(name => join(dir, name)).find(existsSync);
        if (!license) throw new Error(`배포 라이선스 원문을 찾지 못했습니다: ${pkg.name}`);
        packages.set(`${pkg.name}@${pkg.version}`, `${pkg.name}@${pkg.version} (${pkg.license || 'see license'})\n${readFileSync(license, 'utf8')}`);
        break;
      }
    }
    dir = dirname(dir);
  }
}
writeFileSync(join(root, 'ui/generated/THIRD_PARTY_LICENSES.txt'), [...packages].sort(([a], [b]) => a.localeCompare(b)).map(([, notice]) => notice).join('\n\n-----\n\n'));

// A parser-blocking, same-origin resource keeps the catalog available to every
// window, including the small native overlays that do not load an ES module.
const catalog = JSON.parse(readFileSync(join(root, 'ui/messages.json'), 'utf8'));
for (const [key, translations] of Object.entries(catalog)) {
  const languages = Object.keys(translations).sort();
  if (languages.join(',') !== 'en,ja,ko') throw new Error(`Missing UI translation: ${key}`);
  const variables = text => [...text.matchAll(/\{(\w+)\}/g)].map(match => match[1]).sort().join(',');
  for (const [language, value] of Object.entries(translations)) {
    if (typeof value !== 'string' || !value.trim() || variables(value) !== variables(key)) {
      throw new Error(`Invalid ${language} translation or variables: ${key}`);
    }
  }
}
writeFileSync(join(root, 'ui/generated/messages.js'), 'window.jajanMessages = ' + JSON.stringify(catalog) + ';\n');
