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

console.log('--- 2. Regresión visual del escáner estable del 11-03-2026 ---');

const cssMatch = indexSource.match(/<style>([\s\S]*?)<\/style>/i);
assert.ok(cssMatch, 'Debe existir bloque <style>');
const css = cssMatch[1];

function cssRule(selector) {
    const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    return (css.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`, 'i')) || [, ''])[1];
}

const scannerWrapCss = cssRule('.scanner-wrap');
const qrReaderCss = cssRule('#qr-reader');
assert.match(scannerWrapCss, /max-width:\s*400px/i,
    'el contenedor debe conservar el ancho estable de 400px');
assert.doesNotMatch(scannerWrapCss, /aspect-ratio|overflow/i,
    'el contenedor no debe recortar ni forzar un visor cuadrado');
assert.match(qrReaderCss, /max-width:\s*400px/i);
assert.match(qrReaderCss, /min-height:\s*300px/i);
assert.match(qrReaderCss, /border-radius:\s*15px/i);
assert.match(qrReaderCss, /overflow:\s*hidden/i,
    '#qr-reader debe conservar la geometría que funcionaba en marzo');
assert.doesNotMatch(css, /#qr-reader\s+video\s*\{/i,
    'la aplicación no debe forzar tamaño, object-fit ni reproducción del video de la biblioteca');
assert.doesNotMatch(css, /#qr-reader__scan_region\s*\{/i,
    'la aplicación no debe alterar la región de escaneo generada por la biblioteca');
assert.doesNotMatch(indexSource, /scanner-guia|scanner-hint/i,
    'no deben reaparecer overlays añadidos después de la versión estable');

console.log('✓ Geometría nativa del visor estable restaurada');

console.log('--- 3. Regresión de configuración y ciclo de vida de marzo ---');

const inicializarScannerBody = extractFunction(inlineSource, 'inicializarScanner');
assert.match(inicializarScannerBody, /new Html5Qrcode\("qr-reader"\)\s*;/,
    'Html5Qrcode debe construirse sin opciones experimentales ni filtros posteriores');
assert.match(inicializarScannerBody,
    /const config = \{ fps: 10, qrbox: \{ width: 220, height: 220 \}, aspectRatio: 1\.0 \};/,
    'debe conservar exactamente fps, qrbox y aspectRatio de f4a19c5');
assert.match(inicializarScannerBody,
    /\.start\(\{ facingMode: "environment" \}, config, onScanSuccess, onScanError\)/,
    'debe solicitar la cámara trasera con la misma llamada estable');
assert.doesNotMatch(inicializarScannerBody,
    /videoConstraints|width:\s*\{\s*ideal|height:\s*\{\s*ideal|applyConstraints|focusMode|playsinline|webkit-playsinline/,
    'no debe negociar resolución, foco ni reproducción del video fuera de html5-qrcode');
assert.doesNotMatch(inicializarScannerBody,
    /getState|\.pause\(|\.resume\(|formatsToSupport|experimentalFeatures|getCameras/,
    'el arranque no debe añadir estados, formatos ni sondeos posteriores a marzo');

const inicializarAppBody = extractFunction(inlineSource, 'inicializarApp');
assert.doesNotMatch(inicializarAppBody, /inicializarScanner\(/,
    'el primer arranque debe seguir ocurriendo desde el botón Activar Cámara');

const onScanSuccessBody = extractFunction(inlineSource, 'onScanSuccess');
assert.doesNotMatch(onScanSuccessBody, /\.pause\(|\.resume\(|getState|Html5QrcodeScannerState/,
    'la lectura no debe pausar el stream antes de que el modal lo detenga');

const lifecycleStart = inlineSource.indexOf('// ===== EVENTOS DEL SCANNER =====');
const lifecycleEnd = inlineSource.indexOf('// ===== BOTÓN ATRÁS DEL SISTEMA =====', lifecycleStart);
assert.ok(lifecycleStart !== -1 && lifecycleEnd > lifecycleStart, 'Debe existir el bloque de ciclo de vida del escáner');
const lifecycle = inlineSource.slice(lifecycleStart, lifecycleEnd);
assert.match(lifecycle, /html5QrCode\.stop\(\)/,
    'los modales deben destruir el stream con stop, como en marzo');
assert.doesNotMatch(lifecycle, /\.pause\(|\.resume\(|reanudarScannerSiPausado/,
    'no debe persistir el ciclo pause/resume agregado después');
assert.match(lifecycle, /setTimeout\(\(\) => \{\s*inicializarScanner\(\);\s*\}, 300\);/,
    'el escáner debe reiniciarse 300ms después de cerrar el modal');
assert.doesNotMatch(inlineSource,
    /solicitarWakeLock|liberarWakeLock|verificarYReanimarScanner|ultimoTiempoVideo|Html5QrcodeScannerState/,
    'no debe quedar maquinaria de keep-alive, watchdog o estados de cámara posterior a marzo');

console.log('✓ Configuración y ciclo stop/start de f4a19c5 restaurados');
console.log('\nTODOS LOS TESTS DE INTEGRIDAD DEL ESCÁNER PASARON EXITOSAMENTE.');
