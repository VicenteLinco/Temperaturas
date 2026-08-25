const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

const indexSource = fs.readFileSync(new URL('../public/index.html', `file://${__dirname}/`), 'utf8');
const inlineSource = [...indexSource.matchAll(/<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/gi)].at(-1)[1];

function extractFunction(source, name) {
    const pattern = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`, 'g');
    const match = pattern.exec(source);
    if (!match) throw new Error(`Function ${name} not found`);
    const start = match.index;
    const brace = source.indexOf('{', pattern.lastIndex);
    let depth = 0;
    let state = 'code';
    for (let i = brace; i < source.length; i++) {
        const c = source[i];
        const next = source[i + 1];
        if (state === 'line') { if (c === '\n') state = 'code'; continue; }
        if (state === 'block') { if (c === '*' && next === '/') { state = 'code'; i++; } continue; }
        if (state === 'single' || state === 'double' || state === 'template') {
            if (c === '\\') { i++; continue; }
            if ((state === 'single' && c === "'") || (state === 'double' && c === '"') || (state === 'template' && c === '`')) state = 'code';
            continue;
        }
        if (c === '/' && next === '/') { state = 'line'; i++; continue; }
        if (c === '/' && next === '*') { state = 'block'; i++; continue; }
        if (c === "'") { state = 'single'; continue; }
        if (c === '"') { state = 'double'; continue; }
        if (c === '`') { state = 'template'; continue; }
        if (c === '{') depth++;
        if (c === '}' && --depth === 0) return source.slice(start, i + 1);
    }
    throw new Error(`Function ${name} is not balanced`);
}

const extraerTermometroIdCode = extractFunction(inlineSource, 'extraerTermometroId');

const context = {
    URL,
    console
};
vm.createContext(context);
vm.runInContext(extraerTermometroIdCode, context);
const extraer = (str) => vm.runInContext(`extraerTermometroId(${JSON.stringify(str)})`, context);

console.log('--- 1. Pruebas de Extracción de ID en Códigos QR ---');

// 1. Números puros (los que imprime el sistema)
assert.equal(extraer('5'), 5, 'Debe extraer número simple');
assert.equal(extraer('005'), 5, 'Debe extraer número con ceros a la izquierda');
assert.equal(extraer(' 12 '), 12, 'Debe ignorar espacios');
assert.equal(extraer('9999'), 9999, 'Debe extraer IDs altos');

// 2. Parámetros clave-valor
assert.equal(extraer('id=5'), 5);
assert.equal(extraer('termometro=12'), 12);
assert.equal(extraer('t=42'), 42);
assert.equal(extraer('termometro_id=7'), 7);

// 3. Prefijos
assert.equal(extraer('T-5'), 5);
assert.equal(extraer('ID: 15'), 15);
assert.equal(extraer('TERMOMETRO #3'), 3);

// 4. URLs
assert.equal(extraer('https://control.empresa.cl/?id=5'), 5);
assert.equal(extraer('https://control.empresa.cl/?termometro=18'), 18);
assert.equal(extraer('http://localhost:3000/termometros/7'), 7);

// 5. Casos inválidos (debe retornar NaN)
assert.ok(isNaN(extraer('')));
assert.ok(isNaN(extraer(null)));
assert.ok(isNaN(extraer('codigo_barras_1234567890123')));
assert.ok(isNaN(extraer('Sala de Maquinas')));
assert.ok(isNaN(extraer('https://google.com')));

console.log('✓ 15 aserciones de extracción QR pasadas');

console.log('--- 2. Pruebas de Invariantes CSS para WebKit (iOS/Safari) ---');

// Extraer bloque de estilos
const cssMatch = indexSource.match(/<style>([\s\S]*?)<\/style>/i);
assert.ok(cssMatch, 'Debe existir bloque <style>');
const css = cssMatch[1];

// A) No debe tener translateZ(0) en #qr-reader video (provoca pantalla negra en Safari WebKit)
assert.ok(!css.includes('transform: translateZ(0)'),
    'CSS no debe forzar translateZ(0) en video porque desacopla el render de WebKit');

// B) No debe tener display: none !important en #qr-shaded-region
assert.ok(!css.includes('#qr-shaded-region { display: none !important; }'),
    'CSS no debe ocultar #qr-shaded-region porque rompe el cálculo de coordenadas de html5-qrcode');

// C) #qr-reader debe tener borde y posición definida
assert.ok(css.includes('#qr-reader'), 'CSS debe definir #qr-reader');
assert.ok(!css.match(/#qr-reader\s*\{[^}]*aspect-ratio:\s*1\s*\/\s*1/i),
    '#qr-reader no debe forzar aspect-ratio 1/1 que causa colapso de altura en video WebKit');

console.log('✓ Invariantes CSS de renderizado móvil pasadas');

console.log('--- 3. Pruebas de Configuración del Escáner y Keep-Alive ---');

// A) El escáner debe usar qrbox para enmarcar el objetivo
assert.ok(inlineSource.includes('qrbox:'), 'Configuración del escáner debe incluir qrbox');

// B) No debe incluir aspectRatio en config de start (provoca pantalla congelada en iOS Safari)
assert.ok(!inlineSource.includes('aspectRatio: 1.0') && !inlineSource.includes('aspectRatio: 1,'),
    'scanConfig no debe incluir aspectRatio como restricción de hardware');

// C) Debe incluir wakeLock para evitar apagado de pantalla
assert.ok(inlineSource.includes('solicitarWakeLock'), 'Debe existir función solicitarWakeLock');
assert.ok(inlineSource.includes('liberarWakeLock'), 'Debe existir función liberarWakeLock');

// D) Debe escuchar visibilitychange y focus para reanimación automática
assert.ok(inlineSource.includes("addEventListener('visibilitychange'"), 'Debe escuchar visibilitychange');
assert.ok(inlineSource.includes("addEventListener('focus'"), 'Debe escuchar focus');

// E) Modal debe pausar (pause) y no destruir (stop) en show.bs.modal para reanudación instantánea
const modalShowMatch = inlineSource.match(/document\.getElementById\('registroModal'\)\.addEventListener\('show\.bs\.modal',\s*\(\)\s*=>\s*\{([\s\S]*?)\}\);/);
assert.ok(modalShowMatch, 'Debe existir listener show.bs.modal');
assert.ok(modalShowMatch[1].includes('html5QrCode.pause(true)'),
    'show.bs.modal debe pausar el escáner (pause) y no destruirlo (stop)');

console.log('✓ Invariantes de ciclo de vida, WakeLock y keep-alive pasadas');
console.log('\nTODOS LOS TESTS DE INTEGRIDAD DEL ESCÁNER PASARON EXITOSAMENTE.');
