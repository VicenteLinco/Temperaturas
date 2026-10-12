const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

const source = fs.readFileSync('public/admin.html', 'utf8');
const start = source.indexOf('        async function cargarMapaAdmin()');
const end = source.indexOf('        async function guardarConfigMapa()', start);
const functions = source.slice(start, end);

function harness() {
    const elements = {};
    const alerts = [];
    let data = { habilitado: true, imagen_url: '/uploads/mapa.png', areas: [], termometros: [] };
    let ok = true;
    let cached = false;
    const context = {
        mapaData: null, mapaPos: {}, mapaZonas: {}, mapaPendienteSel: 1,
        document: { getElementById(id) {
            if (!elements[id]) {
                elements[id] = {
                    style: {}, naturalWidth: 0, naturalHeight: 0, complete: false,
                    textContent: '', removeAttribute(name) {
                        if (name === 'src') {
                            this.url = '';
                            if (!Object.getOwnPropertyDescriptor(this, 'src')?.set) delete this.src;
                            this.naturalWidth = 0; this.naturalHeight = 0;
                        }
                        else delete this[name];
                    }
                };
                if (id === 'mapaEditorImg') Object.defineProperty(elements[id], 'src', {
                    configurable: true,
                    get() { return this.url; },
                    set(url) {
                        this.url = url;
                        this.complete = cached;
                        this.naturalWidth = cached ? 1200 : 0;
                        this.naturalHeight = cached ? 800 : 0;
                    }
                });
            }
            return elements[id];
        } },
        fetch: async () => ({ ok, json: async () => structuredClone(data) }),
        alert: message => alerts.push(message), console,
        cancelarModoTrazarZona() {}, cerrarBannerPostTrazo() {}, finalizarModoAjustarEsquinas() {},
        resetearZoomMapaAdmin() {}, renderMapaAdmin() {}
    };
    vm.createContext(context);
    vm.runInContext(functions, context);
    return {
        context, elements, alerts,
        load: () => context.cargarMapaAdmin(),
        setData: value => { data = value; },
        setCached: value => { cached = value; },
        failApi: () => { ok = false; }
    };
}

(async () => {
    for (const id of ['mapaTrazarBanner', 'mapaPostTrazoBanner', 'mapaAjustarBanner']) {
        const tag = source.match(new RegExp('<div id="' + id + '"[^>]*>'))[0];
        assert.ok(!/\bd-flex\b/.test(tag), id + ' must respect inline display:none');
    }
    const h = harness();
    await h.load();
    assert.equal(h.elements.mapaEditorCard.style.display, 'none', 'wait for image dimensions before showing editor');
    assert.equal(h.elements.mapaPreview.style.display, 'none');
    const img = h.elements.mapaEditorImg;
    img.naturalWidth = 1200;
    img.naturalHeight = 800;
    img.onload();
    assert.equal(h.elements.mapaEditor.style.aspectRatio, '1200 / 800');
    assert.equal(h.elements.mapaEditorCard.style.display, 'block');
    assert.equal(h.elements.mapaPreview.style.display, 'block');
    assert.equal(h.elements.mapaSinFoto.style.display, 'none');

    h.setCached(true);
    await h.load();
    assert.equal(h.elements.mapaEditorCard.style.display, 'block', 'cached image opens editor immediately');
    assert.equal(h.elements.mapaEditor.style.aspectRatio, '1200 / 800');

    h.setCached(false);
    await h.load();
    const oldLoad = img.onload;
    img.onerror();
    assert.equal(h.elements.mapaEditorCard.style.display, 'none');
    assert.equal(h.elements.mapaPreview.style.display, 'none');
    assert.equal(h.elements.mapaEditor.style.aspectRatio, '');
    assert.match(h.elements.mapaSinFoto.textContent, /No se pudo cargar/);
    assert.equal(h.elements.mapaSinFoto.style.display, 'block');
    assert.ok(!h.elements.mapaPreview.src, 'broken preview must be cleared');

    h.setData({ habilitado: false, imagen_url: null, areas: [], termometros: [] });
    await h.load();
    oldLoad();
    assert.equal(h.elements.mapaEditorCard.style.display, 'none', 'old image callback cannot restore an absent plan');
    assert.equal(h.elements.mapaPreview.style.display, 'none');
    assert.equal(h.elements.mapaEditor.style.aspectRatio, '');
    assert.match(h.elements.mapaSinFoto.textContent, /Sube la foto/);

    h.setData({ habilitado: true, imagen_url: '/uploads/mapa.png', areas: [], termometros: [] });
    h.setCached(true);
    await h.load();
    assert.equal(h.elements.mapaEditorCard.style.display, 'block', 'navigation reload recovers editor');
    h.failApi();
    await h.load();
    assert.equal(h.elements.mapaEditorCard.style.display, 'none', 'API failure must not leave previous editor visible');
    assert.equal(h.elements.mapaPreview.style.display, 'none');
    console.log('admin-mapa: hidden banners, loaded/cached/broken/absent images and reload passed');
})().catch(error => { console.error(error); process.exitCode = 1; });
