const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const OperatorFlow = require('../public/operator-flow.js');

const indexSource = fs.readFileSync(new URL('../public/index.html', `file://${__dirname}/`), 'utf8');
const inlineSource = [...indexSource.matchAll(/<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/gi)].at(-1)[1];
const lecturasSource = fs.readFileSync(new URL('../public/lecturas.js', `file://${__dirname}/`), 'utf8');

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

function element(value = '') {
    const classes = new Set();
    return {
        value, disabled: false, innerHTML: '', textContent: '', href: '', target: '', rel: '',
        style: {}, attributes: {}, className: '',
        classList: {
            add: (...names) => names.forEach(n => classes.add(n)),
            remove: (...names) => names.forEach(n => classes.delete(n)),
            toggle: (name, force) => force ? classes.add(name) : classes.delete(name),
            contains: name => classes.has(name)
        },
        setAttribute(name, value) { this.attributes[name] = value; },
        focus() { this.focused = true; },
        blur() {}, select() {}
    };
}

const functions = [
    'cargarTodasLasAreas', 'leerLecturas', 'formatearFechaHoy', 'totalesRonda',
    'recopilarIncidenciasRonda', 'fechaOperativaActual', 'claveCierreRonda',
    'verificarCierreRonda', 'abrirCierreRonda', 'renderCierreRonda',
    'textoResumenRonda', 'guardarRegistro', 'actualizarUIHumedad',
    'seleccionarIncidenciaHumedad'
].map(name => extractFunction(inlineSource, name)).join('\n');
const sharedFunctions = extractFunction(lecturasSource, 'sanearDecimal');

function response(data, ok = true, status = 200) {
    return { ok, status, json: async () => structuredClone(data), text: async () => '' };
}

function createHarness({ areas, areaResponses, failAreaId = null }) {
    const elements = {
        tempActual: element('8'), lectura2: element('6'), lectura3: element('2'),
        humedad: element(''), humedadGroup: element(), btnGuardar: element(),
        termometroId: element('1'), observaciones: element(''),
        esEdicion: element('false'), registroId: element(''), registroModal: element(),
        btnHumedadLow: element(), btnHumedadError: element(), humedadAyuda: element(),
        loadingOverlay: element(), loadingProgressBar: element(), loadingStatus: element(),
        cierreRondaModal: element(), cierreTitulo: element(), cierreSubtitulo: element(),
        cierreBody: element(), btnWhatsAppRonda: element()
    };
    elements.humedadGroup.style.display = 'none';
    const storage = new Map();
    const calls = { modalShows: 0, confirmations: 0, fetches: 0, payload: null };
    const activeAreas = Object.entries(areaResponses).map(([id, data]) => ({
        id: Number(id), nombre: data.area.nombre, activa: true
    }));
    const context = {
        OperatorFlow, areasData: structuredClone(areas), cargaGlobalCompleta: true,
        humedadIncidencia: null, equiposFueraServicio: [],
        estadoVentana: { activa: true, proximaApertura: null },
        navigator: { onLine: true }, Date, encodeURIComponent,
        document: { getElementById: id => elements[id] || element() },
        localStorage: {
            getItem: key => storage.get(key) || null,
            setItem: (key, value) => storage.set(key, value)
        },
        bootstrap: {
            Modal: class {
                static getInstance() { return { hide() {} }; }
                show() { calls.modalShows++; }
            }
        },
        fetch: async (url, options = {}) => {
            calls.fetches++;
            if (url === '/api/registros' && options.method === 'POST') {
                calls.payload = JSON.parse(options.body);
                return response({});
            }
            if (url === '/api/areas') return response(activeAreas);
            const areaMatch = String(url).match(/^\/api\/areas\/(\d+)\/pendientes$/);
            if (areaMatch) {
                const id = Number(areaMatch[1]);
                if (id === failAreaId) return response({}, false, 503);
                const data = areaResponses[id];
                return response({
                    ventana_horaria: data.ventana,
                    pendientes: data.pendientes,
                    completados: data.completados,
                    ventana_activa: true
                });
            }
            if (url === '/api/termometros') return response([]);
            throw new Error(`Unexpected fetch: ${url}`);
        },
        setTimeout: fn => { fn(); return 1; },
        mostrarToast() {}, mostrarAvisoRegistro() {}, volverAlEscaner() {},
        resetBotonRegistrar() {}, renderizarAreas() {},
        mostrarConfirmacion: async () => { calls.confirmations++; return true; },
        console: { ...console, error() {} }
    };
    vm.createContext(context);
    vm.runInContext(`${sharedFunctions}\n${functions}`, context, { filename: 'public/index.html#orchestration' });
    return { context, elements, calls };
}

async function save(harness) {
    await vm.runInContext('guardarRegistro()', harness.context);
}

const completed = (id, name) => ({
    termometro_id: id, termometro_nombre: name, temp_actual: 4,
    temp_maxima: 6, temp_minima: 2, fuera_rango_operativo: false, observaciones: null
});

(async () => {
    const pendingOtherArea = createHarness({
        areas: {
            1: { area: { nombre: 'A' }, ventana: '14:00', pendientes: [{ id: 1 }], completados: [] },
            2: { area: { nombre: 'B' }, ventana: '14:00', pendientes: [{ id: 2 }], completados: [] }
        },
        areaResponses: {
            1: { area: { nombre: 'A' }, ventana: '14:00', pendientes: [], completados: [completed(1, 'T1')] },
            2: { area: { nombre: 'B' }, ventana: '14:00', pendientes: [{ id: 2 }], completados: [] }
        }
    });
    await save(pendingOtherArea);
    assert.deepEqual(pendingOtherArea.calls.payload, {
        termometro_id: 1, temp_actual: 8, temp_maxima: 6, temp_minima: 2,
        humedad: null, observaciones: null
    });
    assert.equal(pendingOtherArea.calls.modalShows, 0, 'must not finish while another area is pending');

    const previousSnapshot = {
        1: { area: { nombre: 'A' }, ventana: '14:00', pendientes: [{ id: 1 }], completados: [] },
        2: { area: { nombre: 'B' }, ventana: '14:00', pendientes: [{ id: 2 }], completados: [] }
    };
    const partialLoad = createHarness({
        areas: previousSnapshot,
        areaResponses: {
            1: { area: { nombre: 'A' }, ventana: '14:00', pendientes: [], completados: [completed(1, 'T1')] },
            2: { area: { nombre: 'B' }, ventana: '14:00', pendientes: [], completados: [completed(2, 'T2')] }
        },
        failAreaId: 2
    });
    await save(partialLoad);
    assert.equal(partialLoad.calls.modalShows, 0, 'must not finish after a failed global refresh');
    assert.equal(partialLoad.context.cargaGlobalCompleta, false);
    assert.deepEqual(JSON.parse(JSON.stringify(partialLoad.context.areasData)), previousSnapshot,
        'failed refresh must preserve the previous areas snapshot atomically');

    const concurrentCompletion = createHarness({
        areas: {
            1: { area: { nombre: 'A' }, ventana: '14:00', pendientes: [{ id: 1 }], completados: [] },
            2: { area: { nombre: 'B' }, ventana: '14:00', pendientes: [{ id: 2 }], completados: [] }
        },
        areaResponses: {
            1: { area: { nombre: 'A' }, ventana: '14:00', pendientes: [], completados: [completed(1, 'T1')] },
            2: { area: { nombre: 'B' }, ventana: '14:00', pendientes: [], completados: [completed(2, 'T2')] }
        }
    });
    await save(concurrentCompletion);
    assert.equal(concurrentCompletion.calls.modalShows, 1,
        'global refresh must observe another operator completing the other area');
    await vm.runInContext('verificarCierreRonda()', concurrentCompletion.context);
    assert.equal(concurrentCompletion.calls.modalShows, 1, 'completion must open exactly once');
    assert.match(concurrentCompletion.elements.btnWhatsAppRonda.href, /^https:\/\/wa\.me\/\?text=/);
    assert.equal(concurrentCompletion.elements.btnWhatsAppRonda.target, '_blank');
    assert.equal(concurrentCompletion.elements.btnWhatsAppRonda.rel, 'noopener noreferrer');

    vm.runInContext("seleccionarIncidenciaHumedad('LOW')", concurrentCompletion.context);
    assert.equal(concurrentCompletion.context.humedadIncidencia, 'LOW');
    vm.runInContext("seleccionarIncidenciaHumedad('LOW')", concurrentCompletion.context);
    assert.equal(concurrentCompletion.context.humedadIncidencia, null);
    vm.runInContext("seleccionarIncidenciaHumedad('ERROR')", concurrentCompletion.context);
    vm.runInContext("seleccionarIncidenciaHumedad('ERROR')", concurrentCompletion.context);
    assert.equal(concurrentCompletion.calls.confirmations, 0, 'LOW/ERROR must never open confirmation');

    assert.equal(vm.runInContext("sanearDecimal('-12,3')", concurrentCompletion.context), '-12.3');
    assert.equal(vm.runInContext("sanearDecimal('4.5')", concurrentCompletion.context), '4.5');
    assert.equal(vm.runInContext("sanearDecimal('-1.2.3')", concurrentCompletion.context), '-1.23');

    console.log('index-orchestration: roles, atomic refresh, concurrent completion, WhatsApp and humidity passed');
})().catch(error => {
    console.error(error);
    process.exitCode = 1;
});
