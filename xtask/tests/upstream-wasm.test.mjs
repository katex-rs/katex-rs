// Requires: wasm-pack build crates/wasm-binding --target web --no-opt --dev
import {readFileSync} from 'node:fs';
import assert from 'node:assert/strict';
import test from 'node:test';
import {initSync, renderToString} from '../../crates/wasm-binding/pkg/katex.js';

initSync({module: readFileSync(new URL('../../crates/wasm-binding/pkg/katex_bg.wasm', import.meta.url))});
const {cases} = JSON.parse(readFileSync(new URL('../../crates/katex/tests/fixtures/upstream.json', import.meta.url), 'utf8'));

// Attribute and CSS declaration order are insignificant; preserve all content.
function normalize(markup) {
  return markup.replace(/<([a-zA-Z0-9]+)([^>]*)>/g, (_, tag, attrs) => {
    const attributes = [...attrs.matchAll(/([^\s"'=<>]+)\s*=\s*"([^"]*)"/g)].map(([, key, value]) => {
      if (key === 'style') {
        const trailing = value.trimEnd().endsWith(';');
        value = value.split(';').map(s => s.trim()).filter(Boolean).sort().join(';') + (trailing ? ';' : '');
      }
      return [key, value];
    }).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0);
    return `<${tag}${attributes.map(([key, value]) => ` ${key}="${value}"`).join('')}${attrs.trimEnd().endsWith('/') ? '/' : ''}>`;
  });
}

for (const {expression, output, displayMode, expected} of cases) {
  test(`${output}, display=${displayMode}: ${expression}`, () => {
    assert.equal(normalize(renderToString(expression, {output, displayMode, strict: 'ignore', trust: true})), normalize(expected));
  });
}

test('options ignore inherited values and getters, including shadowed hasOwnProperty', () => {
  const defaults = renderToString('x');
  const prototype = {displayMode: true, output: 'html', trust: true};
  Object.defineProperty(prototype, 'strict', {get() { throw new Error('inherited getter invoked'); }});
  const options = Object.create(prototype);
  options.hasOwnProperty = null;
  assert.equal(normalize(renderToString('x', options)), normalize(defaults));
  const own = Object.assign(Object.create(null), {output: 'mathml', displayMode: true});
  assert.equal(normalize(renderToString('x', own)), normalize(renderToString('x', {output: 'mathml', displayMode: true})));
});
