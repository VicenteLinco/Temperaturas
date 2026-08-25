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
// El botón de WhatsApp se eliminó a propósito (commit 0d44837, "simplificar flujo
// móvil, remover WhatsApp"); el cierre de ronda ya no debe reintroducirlo.
assert.doesNotMatch(html, /id="btnWhatsAppRonda"/i,
    'el flujo de cierre de ronda ya no debe incluir el botón de WhatsApp (eliminado intencionalmente)');
assert.match(html, /modal-fullscreen-sm-down/);
assert.match(html, /safe-area-inset-bottom/);
console.log(`index-script: ${scripts.length} inline script(s) parsed; mobile capture contract passed`);
