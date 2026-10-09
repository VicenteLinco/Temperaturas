// Mapa de pendientes sobre el plano (T4, registrador).
// Solo LEE GET /api/mapa y REUSA la captura existente
// (seleccionarTermometro / procesarTermometro): no duplica registro.
(function () {
    'use strict';
    var S = { d: null, detalle: null, s: 1, tx: 0, ty: 0, px: 0, py: 0 };
    var ptrs = new Map(), d0 = 0, s0 = 1, movido = 0, ox = 0, oy = 0;

    function $(id) { return document.getElementById(id); }
    function cls(e) { return e === 'completado' ? 'bg-success' : e === 'fuera_de_servicio' ? 'bg-secondary' : 'bg-danger'; }
    function txt(e) { return e === 'completado' ? 'Completado' : e === 'fuera_de_servicio' ? 'Fuera de servicio' : 'Pendiente'; }
    function esc(s) { return String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;'); }

    // GET /api/mapa con la misma sesion (cookies) que el resto de la app.
    async function cargar() {
        var r = await fetch('/api/mapa');
        if (!r.ok) throw new Error('sin mapa');
        S.d = await r.json();
    }

    function aplicar() {
        var el = $('mapaInterior');
        if (el) el.style.transform = 'translate(' + S.tx + '%,' + S.ty + '%) scale(' + S.s + ') translate(' + S.px + 'px,' + S.py + 'px)';
    }
    function baseDetalle(z) {
        var s = Math.min(100 / z.w, 100 / z.h, 4);
        return { s: s, tx: -z.x * s, ty: -z.y * s };
    }
    function reencuadrar() {
        var b = S.detalle ? baseDetalle(S.detalle.zona) : { s: 1, tx: 0, ty: 0 };
        S.s = b.s; S.tx = b.tx; S.ty = b.ty; S.px = 0; S.py = 0;
        aplicar();
    }

    function render() {
        if (!S.d) return;
        var zh = '', ph = '', enDet = !!S.detalle;
        S.d.areas.forEach(function (a) {
            if (!a.zona || (enDet && a.id !== S.detalle.id)) return;
            var z = a.zona;
            zh += '<div data-tap="zona" data-id="' + a.id + '" style="position:absolute;left:' + z.x + '%;top:' + z.y + '%;width:' + z.w + '%;height:' + z.h + '%;border:2px dashed #1d1d1f;background:rgba(29,29,31,.06);border-radius:8px;display:flex;align-items:flex-start;justify-content:center;cursor:pointer;">'
                + '<span class="badge bg-dark mt-1">' + esc(a.nombre) + '</span></div>';
        });
        S.d.termometros.forEach(function (t) {
            if (t.pos_x == null || t.pos_y == null) return; // sin ubicar: solo lista
            if (enDet && t.area_id !== S.detalle.id) return;
            ph += '<button data-tap="punto" data-id="' + t.id + '" aria-label="' + esc(t.nombre) + '" title="' + esc(t.nombre) + ' · ' + txt(t.estado) + '" class="' + cls(t.estado) + ' text-white border border-white" style="position:absolute;left:' + t.pos_x + '%;top:' + t.pos_y + '%;width:44px;height:44px;margin:-22px 0 0 -22px;border-radius:50%;display:flex;align-items:center;justify-content:center;font-size:1.2rem;box-shadow:0 2px 8px rgba(0,0,0,.3);cursor:pointer;"><i class="bi bi-thermometer-half"></i></button>';
        });
        $('mapaZonas').innerHTML = zh;
        $('mapaPuntos').innerHTML = ph;
        $('mapaVolver').style.display = enDet ? '' : 'none';
        $('mapaPopup').style.display = 'none';
    }

    function popup(id) {
        var t = S.d.termometros.find(function (x) { return x.id === Number(id); });
        if (!t) return;
        $('mapaPopup').innerHTML = '<div class="card-body p-2 d-flex align-items-center gap-2">'
            + '<div class="min-width-0 flex-grow-1"><div class="fw-bold text-truncate">' + esc(t.nombre) + '</div>'
            + '<span class="badge ' + cls(t.estado) + '">' + txt(t.estado) + '</span></div>'
            + '<button class="btn btn-primary btn-sm" onclick="MapaVista.registrar(' + t.id + ')">Registrar</button>'
            + '<button class="btn btn-outline-secondary btn-sm" onclick="MapaVista.cerrar()" aria-label="Cerrar">X</button></div>';
        $('mapaPopup').style.display = 'block';
    }

    function toque(el) {
        var tipo = el.getAttribute('data-tap'), id = Number(el.getAttribute('data-id'));
        if (tipo === 'zona') {
            var a = S.d.areas.find(function (x) { return x.id === id; });
            if (a && a.zona) { S.detalle = { id: id, zona: a.zona }; reencuadrar(); render(); }
        } else if (tipo === 'punto') { popup(id); }
    }

    // Pan con 1 dedo + pinch con 2 (Pointer Events); el tap se detecta manual
    // para no disparar tras arrastrar (click fantasma).
    function enlazar() {
        var m = $('mapaMarco');
        m.addEventListener('pointerdown', function (e) {
            // El popup lleva botones nativos: sin captura ni pan para que el click llegue.
            if (e.target && e.target.closest && e.target.closest('#mapaPopup')) return;
            try { m.setPointerCapture(e.pointerId); } catch (_) {}
            ptrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
            if (ptrs.size === 2) {
                var p = Array.from(ptrs.values());
                d0 = Math.hypot(p[0].x - p[1].x, p[0].y - p[1].y); s0 = S.s;
            }
            ox = e.clientX; oy = e.clientY; movido = 0;
        });
        m.addEventListener('pointermove', function (e) {
            if (!ptrs.has(e.pointerId)) return;
            ptrs.set(e.pointerId, { x: e.clientX, y: e.clientY });
            if (ptrs.size === 2) {
                var p = Array.from(ptrs.values());
                var d = Math.hypot(p[0].x - p[1].x, p[0].y - p[1].y);
                if (d0 > 0) { S.s = Math.min(6, Math.max(0.5, s0 * d / d0)); aplicar(); }
                movido = 99;
                return;
            }
            var dx = e.clientX - ox, dy = e.clientY - oy;
            movido += Math.abs(dx) + Math.abs(dy);
            S.px += dx; S.py += dy; ox = e.clientX; oy = e.clientY;
            aplicar();
        });
        function fin(e) {
            if (!ptrs.has(e.pointerId)) return; // gesto sobre el popup: lo maneja el click nativo
            ptrs.delete(e.pointerId);
            if (movido < 10 && ptrs.size === 0) {
                var t = document.elementFromPoint(e.clientX, e.clientY);
                var hit = t && t.closest ? t.closest('[data-tap]') : null;
                if (hit && m.contains(hit)) toque(hit);
            }
        }
        m.addEventListener('pointerup', fin);
        m.addEventListener('pointercancel', function (e) { ptrs.delete(e.pointerId); });
    }

    async function mostrar(v) {
        if (!S.d) return;
        var lista = v !== 'mapa';
        $('areaActualContent').style.display = lista ? '' : 'none';
        $('mapaVista').style.display = lista ? 'none' : '';
        $('btnVistaLista').classList.toggle('active', lista);
        $('btnVistaMapa').classList.toggle('active', !lista);
        if (!lista) { try { await cargar(); } catch (e) {} render(); } // relee estados
    }

    // Tras guardar desde el mapa, el punto quedaría con el color viejo:
    // al cerrarse el modal de registro se releen los estados.
    function refrescarAlCerrarModal() {
        var rm = $('registroModal');
        if (!rm || rm.dataset.mapaHook) return;
        rm.dataset.mapaHook = '1';
        rm.addEventListener('hidden.bs.modal', function () {
            if ($('mapaVista').style.display === 'none') return;
            cargar().then(render, render);
        });
    }

    // Sin foto o deshabilitado: el mapa se oculta (solo lista, sin errores).
    async function init() {
        try {
            await cargar();
            if (!S.d.habilitado || !S.d.imagen_url) return;
            $('mapaImg').src = S.d.imagen_url;
            $('mapaToggle').style.display = '';
            enlazar(); refrescarAlCerrarModal(); render();
        } catch (e) { /* fallback: la lista sigue intacta */ }
    }
    document.addEventListener('DOMContentLoaded', init);

    window.MapaVista = {
        mostrar: mostrar,
        general: function () { S.detalle = null; reencuadrar(); render(); },
        zoom: function (f) { S.s = Math.min(6, Math.max(0.5, S.s * f)); aplicar(); },
        reset: reencuadrar,
        cerrar: function () { $('mapaPopup').style.display = 'none'; },
        // Entra al flujo de captura EXISTENTE para ese termometro id.
        registrar: function (id) {
            $('mapaPopup').style.display = 'none';
            if (typeof seleccionarTermometro === 'function') seleccionarTermometro(id);
            else if (typeof procesarTermometro === 'function') procesarTermometro(id);
        }
    };
})();
