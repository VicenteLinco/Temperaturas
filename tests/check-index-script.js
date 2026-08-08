const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');

const html = fs.readFileSync(new URL('../public/index.html', `file://${__dirname}/`), 'utf8');
const scripts = [...html.matchAll(/<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/gi)];
if (!scripts.length) throw new Error('No inline scripts found in public/index.html');
scripts.forEach((match, index) => new vm.Script(match[1], { filename: `public/index.html#script-${index + 1}` }));

for (const id of ['tempActual', 'lectura2', 'lectura3', 'humedad']) {
    const input = html.match(new RegExp(`<input[^>]*id="${id}"[^>]*>`, 'i'))?.[0];
    assert.ok(input, `missing ${id}`);
    assert.match(input, /type="text"/i, `${id} must open the full keyboard`);
    assert.doesNotMatch(input, /inputmode=/i, `${id} must not force a numeric keyboard`);
}
assert.match(html, /id="btnHumedadLow"/);
assert.match(html, /id="btnHumedadError"/);
const whatsapp = html.match(/<a[^>]*id="btnWhatsAppRonda"[^>]*>/i)?.[0] || '';
assert.match(whatsapp, /href="https:\/\/wa\.me\/"/i);
assert.match(whatsapp, /target="_blank"/i);
assert.match(whatsapp, /rel="noopener noreferrer"/i);
assert.match(html, /modal-fullscreen-sm-down/);
assert.match(html, /safe-area-inset-bottom/);
console.log(`index-script: ${scripts.length} inline script(s) parsed; mobile capture contract passed`);
