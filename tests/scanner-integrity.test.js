const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

const indexSource = fs.readFileSync(new URL('../public/index.html', `file://${__dirname}/`), 'utf8');
const diagnosticSource = fs.readFileSync(new URL('../public/test_scanner.html', `file://${__dirname}/`), 'utf8');
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

console.log('--- 2. Contrato visual mínimo del escáner ---');

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
    'el contenedor debe limitar únicamente el ancho del visor');
assert.doesNotMatch(scannerWrapCss, /aspect-ratio|overflow|border-radius/i,
    'el contenedor no debe recortar, redondear ni forzar la geometría del video');
assert.match(qrReaderCss, /max-width:\s*400px/i);
assert.match(qrReaderCss, /min-height:\s*300px/i);
assert.doesNotMatch(qrReaderCss, /overflow|border-radius/i,
    '#qr-reader no debe recortar el video generado por la biblioteca');
assert.doesNotMatch(css, /#qr-reader\s+video\s*\{/i,
    'la aplicación no debe forzar tamaño, object-fit ni reproducción del video de la biblioteca');
assert.doesNotMatch(css, /#qr-reader__scan_region\s*\{/i,
    'la aplicación no debe alterar la región de escaneo generada por la biblioteca');

console.log('✓ El visor no añade clipping ni geometría sobre el video generado');

console.log('--- 3. Configuración mínima y activación explícita ---');

const inicializarScannerBody = extractFunction(inlineSource, 'inicializarScanner');
assert.match(inicializarScannerBody,
    /const config = \{ fps: 10 \};/,
    'el escáner debe configurar únicamente la frecuencia de lectura');
assert.match(inicializarScannerBody,
    /\.start\(\{ facingMode: "environment" \}, config, onScanSuccess, onScanError\)/,
    'debe solicitar únicamente la cámara trasera sin restricciones adicionales');
assert.doesNotMatch(inicializarScannerBody,
    /aspectRatio|qrbox|videoConstraints|applyConstraints|focusMode|playsinline|webkit-playsinline/,
    'no debe negociar geometría, resolución, foco ni reproducción del video');
assert.doesNotMatch(inicializarScannerBody,
    /getState|\.pause\(|\.resume\(|formatsToSupport|experimentalFeatures|getCameras/,
    'el arranque no debe añadir estados, formatos ni sondeos de cámara');

const inicializarAppBody = extractFunction(inlineSource, 'inicializarApp');
assert.doesNotMatch(inicializarAppBody, /inicializarScanner\(/,
    'la carga de la aplicación no debe solicitar la cámara');
assert.match(indexSource, /onclick="inicializarScanner\(\)"/,
    'Activar Cámara debe ser el único punto explícito de entrada');
const inlineSinInicializador = inlineSource.replace(inicializarScannerBody, '');
assert.doesNotMatch(inlineSinInicializador, /\binicializarScanner\s*\(/,
    'ningún timer, modal o evento puede readquirir la cámara automáticamente');

const onScanSuccessBody = extractFunction(inlineSource, 'onScanSuccess');
assert.doesNotMatch(onScanSuccessBody, /\.pause\(|\.resume\(|getState|Html5QrcodeScannerState/,
    'la lectura no debe manipular el stream antes de que el modal lo detenga');

const reposoBody = extractFunction(inlineSource, 'dejarScannerEnReposo');
const detenerScannerBody = extractFunction(inlineSource, 'detenerInstanciaScanner');
const mostrarOverlayBody = extractFunction(inlineSource, 'mostrarOverlayScanner');
const confirmarReproduccionBody = extractFunction(inlineSource, 'confirmarReproduccionScanner');
assert.equal((detenerScannerBody.match(/\.stop\(\)/g) || []).length, 1,
    'el helper de salida debe contener una sola operación stop');
assert.match(reposoBody, /scannerAttemptGeneration\+\+;/,
    'la salida debe invalidar inmediatamente cualquier arranque pendiente');
assert.match(mostrarOverlayBody, /btn\.disabled = !disponible/,
    'el overlay debe impedir una nueva activación mientras haya trabajo pendiente');
assert.match(mostrarOverlayBody, /Cerrando\.\.\./,
    'la invalidación debe explicar que la cámara todavía se está cerrando');
assert.match(reposoBody, /await operacionAnterior\.catch\(\(\) => \{\}\)/,
    'el cierre debe esperar a que el dueño anterior termine');
assert.match(reposoBody, /scannerPendingOperation = fronteraCierre/,
    'el cierre debe poseer la frontera compartida hasta completar stop');
const modalShowBody = inlineSource.match(
    /document\.querySelectorAll\('\.modal'\)\.forEach\([\s\S]*?addEventListener\('shown\.bs\.modal'/
)?.[0] || '';
assert.equal((modalShowBody.match(/dejarScannerEnReposo\(true\)/g) || []).length, 1,
    'todos los modales deben compartir una sola frontera de salida de cámara');
assert.match(modalShowBody,
    /addEventListener\('show\.bs\.modal'[\s\S]*?dejarScannerEnReposo\(true\)/,
    'cada modal debe detener el escáner exactamente una vez al abrirse');
assert.doesNotMatch(inlineSource,
    /solicitarWakeLock|liberarWakeLock|verificarYReanimarScanner|ultimoTiempoVideo|Html5QrcodeScannerState/,
    'no debe existir keep-alive, watchdog ni una máquina de estados paralela');

assert.match(confirmarReproduccionBody, /await Promise\.resolve\(video\.play\(\)\)/,
    'producción debe observar explícitamente el rechazo de video.play()');
assert.match(confirmarReproduccionBody, /video\.videoWidth > 0 && video\.videoHeight > 0/,
    'producción debe exigir dimensiones reales del stream');
assert.match(confirmarReproduccionBody, /video\.readyState >= 2 && video\.currentTime > tiempoInicial/,
    'producción debe exigir reproducción preparada y avanzando');
assert.match(confirmarReproduccionBody, /Promise\.race\(\[observacion, timeout\]\)/,
    'la confirmación de arranque debe tener un límite acotado');
assert.ok((inicializarScannerBody.match(/generacion !== scannerAttemptGeneration/g) || []).length >= 3,
    'el arranque debe validar propiedad después de start, confirmación y errores');
assert.ok(inicializarScannerBody.indexOf('await confirmarReproduccionScanner()') <
    inicializarScannerBody.indexOf("style.display = 'none'"),
    'el overlay no debe ocultarse antes de confirmar reproducción');

console.log('✓ La cámara se inicia por gesto y nunca se readquiere desde un modal');

console.log('--- 4. Preflight y errores accionables ---');

assert.match(inicializarScannerBody, /typeof window\.Html5Qrcode !== 'function'/,
    'debe detectar que la biblioteca no cargó antes de construir el lector');
assert.match(inicializarScannerBody, /!window\.isSecureContext/,
    'debe rechazar HTTP antes de solicitar la cámara');
assert.match(inicializarScannerBody, /!navigator\.mediaDevices\?\.getUserMedia/,
    'debe detectar navegadores sin getUserMedia');
assert.ok(inicializarScannerBody.indexOf('try {') < inicializarScannerBody.indexOf('new window.Html5Qrcode'),
    'la construcción síncrona debe quedar dentro del try/catch');
assert.match(inicializarScannerBody, /if \(scannerPendingOperation\) return scannerPendingOperation/,
    'una activación no puede comenzar mientras la frontera anterior siga pendiente');
assert.match(inicializarScannerBody, /await detenerInstanciaScanner\(scanner\);\s*mostrarReposoAlFinal = true/,
    'un error debe completar su cleanup antes de solicitar la reactivación del botón');

const errorBody = extractFunction(inlineSource, 'mensajeErrorCamara');
for (const nombre of ['NotAllowedError', 'NotFoundError', 'NotReadableError', 'OverconstrainedError']) {
    assert.match(errorBody, new RegExp(nombre), `debe distinguir ${nombre}`);
}
assert.match(errorBody, /\$\{base\} \(\$\{nombre\}\)/,
    'el mensaje debe exponer el nombre útil del DOMException');

function cargarRuntimeScanner(Html5Qrcode, video = null) {
    const cambiosOverlay = [];
    let display = 'flex';
    const style = {};
    Object.defineProperty(style, 'display', {
        get: () => display,
        set: value => {
            display = value;
            cambiosOverlay.push(value);
        }
    });
    const boton = { disabled: false, innerHTML: '' };
    const overlay = { style, querySelector: () => boton };
    const runtime = {
        window: { isSecureContext: true, Html5Qrcode },
        navigator: { mediaDevices: { getUserMedia() {} } },
        document: {
            querySelector(selector) {
                if (selector === '#scannerOverlay button') return boton;
                if (selector === '#qr-reader video') return typeof video === 'function' ? video() : video;
                return null;
            },
            getElementById: () => overlay
        },
        console: { error() {} },
        estadoMostrado: null,
        Promise,
        setTimeout,
        clearTimeout
    };
    vm.createContext(runtime);
    vm.runInContext(`
        let html5QrCode = null;
        let scannerAttemptGeneration = 0;
        let scannerPendingOperation = null;
        function onScanSuccess() {}
        function onScanError() {}
        function mostrarEstado(mensaje, tipo) { estadoMostrado = { mensaje, tipo }; }
        ${detenerScannerBody}
        ${mostrarOverlayBody}
        ${reposoBody}
        ${confirmarReproduccionBody}
        ${errorBody}
        ${inicializarScannerBody}
    `, runtime);
    return { runtime, boton, overlay, cambiosOverlay };
}

async function probarFalloSinVideoReproducible() {
    let detenciones = 0;
    const scanner = {
        start: async () => {},
        stop: async () => { detenciones++; }
    };
    const video = {
        paused: true,
        play: async () => { video.paused = false; },
        videoWidth: 0,
        videoHeight: 0,
        readyState: 0,
        currentTime: 0,
        offsetWidth: 0,
        offsetHeight: 0,
        getClientRects: () => []
    };
    const { runtime, boton, overlay, cambiosOverlay } = cargarRuntimeScanner(
        function Html5Qrcode() { return scanner; },
        video
    );

    await vm.runInContext('inicializarScanner()', runtime);

    assert.equal(detenciones, 1,
        'start resuelto sin reproducción debe detener el stream incompleto');
    assert.equal(overlay.style.display, 'flex',
        'start resuelto sin reproducción debe conservar el overlay de recuperación');
    assert.equal(boton.disabled, false,
        'start resuelto sin reproducción debe rehabilitar Activar Cámara');
    assert.doesNotMatch(cambiosOverlay.join(','), /none/,
        'no debe declarar éxito ocultando el overlay antes de reproducir');
    assert.equal(runtime.estadoMostrado?.tipo, 'danger');
    assert.match(runtime.estadoMostrado?.mensaje || '', /video no comenzó|VideoPlaybackError/i,
        'debe mostrar un motivo accionable de reproducción fallida');
}

async function probarModalInvalidaStartPendiente() {
    let resolverStart;
    let iniciado = false;
    let detenciones = 0;
    let detencionesCompletadas = 0;
    const scanner = {
        start: () => new Promise(resolve => {
            resolverStart = () => {
                iniciado = true;
                resolve();
            };
        }),
        stop: () => {
            detenciones++;
            if (!iniciado) throw new Error('Scanner is STARTING');
            iniciado = false;
            detencionesCompletadas++;
            return Promise.resolve();
        }
    };
    const { runtime, boton, overlay, cambiosOverlay } = cargarRuntimeScanner(
        function Html5Qrcode() { return scanner; }
    );

    const arranquePendiente = vm.runInContext('inicializarScanner()', runtime);
    const invalidacionPendiente = vm.runInContext('dejarScannerEnReposo(true)', runtime);
    assert.equal(boton.disabled, true,
        'el botón debe seguir bloqueado mientras el start invalidado no termine');
    assert.match(boton.innerHTML, /Cerrando/);
    resolverStart();
    await Promise.all([arranquePendiente, invalidacionPendiente]);

    assert.equal(detenciones, 1,
        'la frontera del modal debe detener una sola vez después del start pendiente');
    assert.equal(detencionesCompletadas, 1,
        'el stream que completó después del modal debe quedar detenido');
    assert.equal(overlay.style.display, 'flex');
    assert.equal(boton.disabled, false);
    assert.doesNotMatch(cambiosOverlay.join(','), /none/,
        'un start invalidado nunca debe ocultar el overlay detrás del modal');
    assert.equal(runtime.estadoMostrado, null,
        'la invalidación intencional no debe publicar un error engañoso');
}

async function probarModalDuranteConfirmacionSerializaNuevoIntento() {
    let inicios = 0;
    let detenciones = 0;
    let streamActivo = false;
    const scanner = {
        start: async () => {
            inicios++;
            streamActivo = true;
        },
        stop: async () => {
            detenciones++;
            streamActivo = false;
        }
    };
    const crearVideoAvanzando = () => {
        const inicio = Date.now();
        return {
            paused: false,
            videoWidth: 1280,
            videoHeight: 720,
            readyState: 4,
            get currentTime() { return (Date.now() - inicio) / 1000; },
            offsetWidth: 320,
            offsetHeight: 240,
            getClientRects: () => [{}]
        };
    };
    let videoActual = crearVideoAvanzando();
    const { runtime, boton, overlay, cambiosOverlay } = cargarRuntimeScanner(
        function Html5Qrcode() { return scanner; },
        () => videoActual
    );

    const primerArranque = vm.runInContext('inicializarScanner()', runtime);
    await new Promise(resolve => setTimeout(resolve, 50));
    const cierreModal = vm.runInContext('dejarScannerEnReposo(true)', runtime);
    const activacionMientrasCierra = vm.runInContext('inicializarScanner()', runtime);

    assert.equal(boton.disabled, true,
        'durante la confirmación invalidada el botón debe permanecer bloqueado');
    assert.match(boton.innerHTML, /Cerrando/);
    await Promise.all([primerArranque, cierreModal, activacionMientrasCierra]);

    assert.equal(inicios, 1,
        'un toque durante el cleanup no debe iniciar un segundo stream');
    assert.equal(detenciones, 1,
        'el cleanup invalidado debe detener exactamente el primer stream');
    assert.equal(streamActivo, false);
    assert.equal(overlay.style.display, 'flex');
    assert.equal(boton.disabled, false,
        'Activar Cámara solo se rehabilita después de completar el cleanup');
    assert.doesNotMatch(cambiosOverlay.join(','), /none/,
        'el intento invalidado no debe ocultar el overlay');

    videoActual = crearVideoAvanzando();
    await vm.runInContext('inicializarScanner()', runtime);
    await new Promise(resolve => setTimeout(resolve, 25));

    assert.equal(inicios, 2,
        'un nuevo toque explícito después del cleanup debe iniciar otro stream');
    assert.equal(detenciones, 1,
        'el dueño obsoleto no debe despertar y detener el stream nuevo');
    assert.equal(streamActivo, true,
        'el stream del segundo intento debe permanecer activo');
    assert.equal(overlay.style.display, 'none');
    assert.equal(runtime.estadoMostrado, null,
        'la serialización normal no debe mostrar un error falso');
}

async function probarFalloSincronicoConstructor() {
    const { runtime, boton } = cargarRuntimeScanner(function Html5Qrcode() {
        const error = new Error('camera busy');
        error.name = 'NotReadableError';
        throw error;
    });

    await vm.runInContext('inicializarScanner()', runtime);
    assert.equal(boton.disabled, false,
        'un fallo síncrono del constructor debe volver a habilitar Activar Cámara');
    assert.equal(runtime.estadoMostrado?.mensaje,
        'La cámara está ocupada por otra aplicación (NotReadableError).');
}

console.log('✓ Los fallos de plataforma dejan una salida manual y un motivo preciso');

console.log('--- 5. Diagnóstico alineado con producción ---');

assert.doesNotMatch(diagnosticSource,
    /videoConstraints|aspectRatio|qrbox|formatsToSupport|#qr-reader-diag\s+video\s*\{|\.play\(|playsinline|webkit-playsinline/i,
    'el diagnóstico no debe forzar cámara, geometría ni reproducción');
assert.match(diagnosticSource, /const scanConfig = \{ fps: 10 \};/,
    'el diagnóstico debe usar la misma configuración mínima');
assert.match(diagnosticSource, /typeof window\.Html5Qrcode !== 'function'/);
assert.match(diagnosticSource, /!window\.isSecureContext \|\| !navigator\.mediaDevices\?\.getUserMedia/);

const observarVideoBody = extractFunction(diagnosticSource, 'observarVideoDiagnostico');
assert.match(observarVideoBody, /video\.videoWidth > 0 && video\.videoHeight > 0/,
    'debe probar dimensiones reales del stream');
assert.match(observarVideoBody, /video\.currentTime > tiempoInicial/,
    'debe comprobar que la reproducción avanza');
assert.match(observarVideoBody, /video\.getClientRects\(\)\.length > 0/,
    'debe comprobar que el video es visible en el layout');
assert.match(diagnosticSource, /if \(estadoVideo\.ok\)/,
    'solo una observación positiva puede declarar éxito');
assert.match(diagnosticSource, /Stream activo: dimensiones válidas y reproducción avanzando\./,
    'el diagnóstico debe describir el estado observado del stream');
assert.doesNotMatch(diagnosticSource, /funcionando al 100%/i,
    'el diagnóstico no debe afirmar que WebKit pintó píxeles que no puede observar');

console.log('✓ El diagnóstico observa dimensiones y avance del stream antes de aprobar');

async function ejecutarRegresionesRuntime() {
    console.log('--- 6. Regresiones de arranque asíncrono ---');
    await probarFalloSinVideoReproducible();
    console.log('✓ start resuelto sin video reproducible conserva una salida manual accionable');
    await probarModalInvalidaStartPendiente();
    console.log('✓ modal durante STARTING invalida y detiene el stream tardío sin falso éxito');
    await probarModalDuranteConfirmacionSerializaNuevoIntento();
    console.log('✓ modal durante confirmación serializa cleanup y protege el siguiente stream');
    await probarFalloSincronicoConstructor();
    console.log('✓ fallo síncrono del constructor rehabilita Activar Cámara');
}

ejecutarRegresionesRuntime()
    .then(() => console.log('\nTODOS LOS TESTS DE INTEGRIDAD DEL ESCÁNER PASARON EXITOSAMENTE.'))
    .catch(error => {
        console.error(error);
        process.exitCode = 1;
    });
