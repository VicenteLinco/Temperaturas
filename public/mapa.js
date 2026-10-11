// Mapa de pendientes sobre el plano (T4, registrador).
// Solo LEE GET /api/mapa y REUSA la captura existente
// (seleccionarTermometro / procesarTermometro): no duplica registro.
(function () {
    'use strict';
    var S = {
        d: null,
        detalle: null,
        subzona: null,
        panelFaltantesAbierto: true,
        s: 1,
        tx: 0,
        ty: 0,
        px: 0,
        py: 0
    };
    var ptrs = new Map(), d0 = 0, s0 = 1, movido = 0, ox = 0, oy = 0;

    function $(id) { return document.getElementById(id); }
    function cls(e) { return e === 'completado' ? 'bg-success' : e === 'fuera_de_servicio' ? 'bg-secondary' : 'bg-primary'; }
    function txt(e) { return e === 'completado' ? 'Completado' : e === 'fuera_de_servicio' ? 'Fuera de servicio' : 'Pendiente'; }
    function esc(s) { return String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;'); }

    // GET /api/mapa con la misma sesión (cookies) que el resto de la app.
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

        // Estadísticas por área (totales, pendientes, etc.)
        var statsPorArea = {};
        S.d.areas.forEach(function (a) {
            var termos = S.d.termometros.filter(function (t) { return t.area_id === a.id; });
            statsPorArea[a.id] = {
                total: termos.length,
                pendientes: termos.filter(function (t) { return t.estado === 'pendiente'; }).length,
                completados: termos.filter(function (t) { return t.estado === 'completado'; }).length,
                fueraServicio: termos.filter(function (t) { return t.estado === 'fuera_de_servicio'; }).length
            };
        });

        // 1. Zonas en el plano
        S.d.areas.forEach(function (a) {
            if (!a.zona) return;
            if (enDet && a.id !== S.detalle.id) return;
            var z = a.zona;
            var st = statsPorArea[a.id] || { pendientes: 0, total: 0 };
            var tienePend = st.pendientes > 0;

            if (enDet) {
                // En vista detallada de la zona: borde sutil
                zh += '<div style="position:absolute;left:' + z.x + '%;top:' + z.y + '%;width:' + z.w + '%;height:' + z.h + '%;border:2px dashed rgba(29,29,31,.3);border-radius:10px;pointer-events:none;"></div>';
            } else {
                // En vista general: etiqueta con conteo de faltantes y estilo interactivo
                var borderStyle = tienePend
                    ? 'border:2px dashed #0071e3;background:rgba(0,113,227,.09);'
                    : 'border:2px dashed #1d1d1f;background:rgba(29,29,31,.05);';

                zh += '<div data-tap="zona" data-id="' + a.id + '" class="mapa-zona-rect" style="position:absolute;left:' + z.x + '%;top:' + z.y + '%;width:' + z.w + '%;height:' + z.h + '%;' + borderStyle + 'border-radius:10px;display:flex;flex-direction:column;align-items:center;justify-content:flex-start;padding-top:4px;cursor:pointer;">'
                    + '<span class="badge bg-dark mb-1 shadow-xs">' + esc(a.nombre) + '</span>'
                    + (tienePend
                        ? '<span class="badge bg-primary text-white shadow-xs" style="font-size:0.75rem;"><i class="bi bi-clock-history me-1"></i>' + st.pendientes + ' por registrar</span>'
                        : '<span class="badge bg-success text-white shadow-xs" style="font-size:0.72rem;">✓ Al día</span>')
                    + '</div>';
            }
        });

        // 2. Pines de termómetros
        S.d.termometros.forEach(function (t) {
            if (t.pos_x == null || t.pos_y == null) return; // sin ubicar: solo en lista
            if (enDet && t.area_id !== S.detalle.id) return;
            if (enDet && S.subzona && (t.ubicacion || '').trim() !== S.subzona) return;

            var ubicTexto = t.ubicacion ? ' (' + esc(t.ubicacion) + ')' : '';
            var aria = esc(t.nombre || 'Termómetro #' + t.id) + ubicTexto + ' · ' + txt(t.estado);

            ph += '<button data-tap="punto" data-id="' + t.id + '" id="pin-term-' + t.id + '" aria-label="' + aria + '" title="' + aria + '" class="' + cls(t.estado) + ' text-white border border-white" style="position:absolute;left:' + t.pos_x + '%;top:' + t.pos_y + '%;width:44px;height:44px;margin:-22px 0 0 -22px;border-radius:50%;display:flex;align-items:center;justify-content:center;font-size:0.95rem;font-weight:700;box-shadow:0 2px 8px rgba(0,0,0,.3);cursor:pointer;">' + esc(t.id) + '</button>';
        });

        $('mapaZonas').innerHTML = zh;
        $('mapaPuntos').innerHTML = ph;

        // 3. Controles de salida y banner de zona
        var btnVolver = $('mapaVolver');
        if (btnVolver) btnVolver.style.display = enDet ? '' : 'none';

        var btnFlot = $('mapaBtnFlotanteSalir');
        if (btnFlot) btnFlot.style.display = enDet ? '' : 'none';

        var banActiva = $('mapaZonaActivaBanner');
        if (banActiva) {
            if (enDet) {
                var currArea = S.d.areas.find(function (x) { return x.id === S.detalle.id; });
                var st = statsPorArea[S.detalle.id] || { pendientes: 0, total: 0 };
                var nomEl = $('mapaNombreZonaActiva');
                var infoEl = $('mapaInfoZonaActiva');
                if (nomEl) nomEl.textContent = currArea ? currArea.nombre : '';
                if (infoEl) {
                    infoEl.innerHTML = '<span class="badge ' + (st.pendientes > 0 ? 'bg-primary' : 'bg-success') + ' ms-2">'
                        + (st.pendientes > 0 ? st.pendientes + ' de ' + st.total + ' por registrar' : 'Todos al día (' + st.total + ')')
                        + '</span>';
                }
                banActiva.style.display = 'flex';
            } else {
                banActiva.style.display = 'none';
            }
        }

        var p = $('mapaPopup');
        if (p) p.style.display = 'none';

        // 4. Selector de Área
        var sel = $('mapaSelectArea');
        if (sel) {
            var curr = S.detalle ? String(S.detalle.id) : '';
            var optHtml = '<option value="">Todas las zonas</option>';
            S.d.areas.forEach(function (a) {
                if (!a.zona) return;
                var st = statsPorArea[a.id] || { pendientes: 0 };
                optHtml += '<option value="' + a.id + '"' + (String(a.id) === curr ? ' selected' : '') + '>'
                    + esc(a.nombre) + (st.pendientes > 0 ? ' (' + st.pendientes + ' pend.)' : ' (al día)') + '</option>';
            });
            sel.innerHTML = optHtml;
        }

        // 5. Selector de Subzona (ubicaciones dentro de la zona activa)
        var selSub = $('mapaSelectSubzona');
        if (selSub) {
            if (enDet) {
                var termosArea = S.d.termometros.filter(function (t) { return t.area_id === S.detalle.id && t.ubicacion && t.ubicacion.trim(); });
                var subzonasSet = {};
                termosArea.forEach(function (t) { subzonasSet[t.ubicacion.trim()] = true; });
                var subzonasUnicas = Object.keys(subzonasSet).sort();

                if (subzonasUnicas.length > 0) {
                    var subHtml = '<option value="">Todas las subzonas</option>';
                    subzonasUnicas.forEach(function (sz) {
                        subHtml += '<option value="' + esc(sz) + '"' + (S.subzona === sz ? ' selected' : '') + '>Subzona: ' + esc(sz) + '</option>';
                    });
                    selSub.innerHTML = subHtml;
                    selSub.style.display = '';
                } else {
                    selSub.style.display = 'none';
                }
            } else {
                selSub.style.display = 'none';
            }
        }

        // 6. Barra de acceso rápido (chips de zonas)
        var zb = $('mapaZonasBar');
        if (zb) {
            var zbh = '<button class="btn btn-sm ' + (!enDet ? 'btn-dark' : 'btn-outline-dark') + ' py-1 px-2.5 rounded-pill" onclick="MapaVista.general()"><i class="bi bi-building me-1"></i>General</button>';
            S.d.areas.forEach(function (a) {
                if (!a.zona) return;
                var st = statsPorArea[a.id] || { pendientes: 0 };
                var activa = enDet && S.detalle.id === a.id;
                var badgeCls = st.pendientes > 0 ? 'bg-primary' : 'bg-success';
                zbh += '<button class="btn btn-sm ' + (activa ? 'btn-dark' : 'btn-outline-dark') + ' py-1 px-2.5 rounded-pill d-inline-flex align-items-center gap-1.5" onclick="MapaVista.seleccionarArea(' + a.id + ')">'
                    + '<span>' + esc(a.nombre) + '</span>'
                    + '<span class="badge ' + badgeCls + ' rounded-pill" style="font-size:0.7rem;">' + st.pendientes + '</span>'
                    + '</button>';
            });
            zb.innerHTML = zbh;
        }

        // 7. Panel de Faltantes ("Cuáles faltan y en qué parte")
        renderPanelFaltantes();
    }

    function renderPanelFaltantes() {
        var pFalt = $('mapaPanelFaltantes');
        if (!pFalt || !S.d) return;

        var enDet = !!S.detalle;
        var listaPendientes = S.d.termometros.filter(function (t) {
            if (t.estado !== 'pendiente') return false;
            if (enDet && t.area_id !== S.detalle.id) return false;
            if (enDet && S.subzona && (t.ubicacion || '').trim() !== S.subzona) return false;
            return true;
        });

        var areaMap = {};
        S.d.areas.forEach(function (a) { areaMap[a.id] = a.nombre; });

        var fHtml = '<div class="card border-0 shadow-sm" style="border-radius:12px;overflow:hidden;">'
            + '<div class="card-header bg-white py-2 px-3 d-flex justify-content-between align-items-center border-bottom">'
            + '<div class="d-flex align-items-center gap-2">'
            + '<i class="bi bi-clipboard-check text-primary"></i>'
            + '<strong class="small text-dark">' + (enDet ? 'Faltan en esta zona' : 'Faltan en la planta') + '</strong>'
            + '<span class="badge ' + (listaPendientes.length > 0 ? 'bg-primary' : 'bg-success') + ' rounded-pill">' + listaPendientes.length + '</span>'
            + '</div>'
            + '<button class="btn btn-sm btn-link text-decoration-none text-muted py-0" onclick="MapaVista.toggleFaltantes()">'
            + (S.panelFaltantesAbierto ? '<i class="bi bi-chevron-up"></i>' : '<i class="bi bi-chevron-down"></i>')
            + '</button>'
            + '</div>';

        if (S.panelFaltantesAbierto) {
            fHtml += '<div class="card-body p-2" style="max-height:300px;overflow-y:auto;">';
            if (listaPendientes.length === 0) {
                fHtml += '<div class="text-center py-3 text-success">'
                    + '<i class="bi bi-check-circle fs-4 d-block mb-1"></i>'
                    + '<span class="small fw-semibold">¡Todos los termómetros de esta vista están al día!</span>'
                    + '</div>';
            } else {
                fHtml += '<div class="d-flex flex-column gap-2">';
                listaPendientes.forEach(function (t) {
                    var aNom = areaMap[t.area_id] || 'Área ' + t.area_id;
                    var tienePos = t.pos_x != null && t.pos_y != null;

                    fHtml += '<div class="d-flex align-items-center justify-content-between p-2 rounded border bg-light gap-2 flex-wrap" style="border-radius:8px !important;">'
                        + '<div class="d-flex align-items-center gap-2">'
                        + '<span class="badge bg-primary text-white rounded-pill px-2 py-1" style="font-size:0.8rem;">#' + esc(t.id) + '</span>'
                        + '<div>'
                        + '<div class="fw-bold small text-dark">' + esc(t.nombre || 'Termómetro #' + t.id) + '</div>'
                        + '<div class="text-muted" style="font-size:0.75rem;">'
                        + '<i class="bi bi-geo-alt"></i> Zona: <strong>' + esc(aNom) + '</strong>'
                        + (t.ubicacion ? ' · Subzona/Parte: <strong>' + esc(t.ubicacion) + '</strong>' : '')
                        + '</div>'
                        + '</div>'
                        + '</div>'
                        + '<div class="d-flex align-items-center gap-1.5 ms-auto">'
                        + (tienePos
                            ? '<button type="button" class="btn btn-sm btn-outline-primary py-1 px-2" style="font-size:0.78rem;" onclick="MapaVista.ubicarEnPlano(' + t.id + ')"><i class="bi bi-crosshair me-1"></i>Ubicar</button>'
                            : '<span class="badge bg-secondary py-1 px-1.5" style="font-size:0.7rem;"><i class="bi bi-pin-map-fill me-1"></i>Sin plano</span>')
                        + '<button type="button" class="btn btn-sm btn-primary py-1 px-2.5" style="font-size:0.78rem;" onclick="MapaVista.registrar(' + t.id + ')"><i class="bi bi-pencil-square me-1"></i>Registrar</button>'
                        + '</div>'
                        + '</div>';
                });
                fHtml += '</div>';
            }
            fHtml += '</div>';
        }
        fHtml += '</div>';
        pFalt.innerHTML = fHtml;
    }

    // Apertura directa: el punto abre el registro EXISTENTE sin popup intermedio.
    function abrirRegistro(id) {
        var p = $('mapaPopup');
        if (p) p.style.display = 'none';
        if (typeof seleccionarTermometro === 'function') seleccionarTermometro(Number(id));
        else if (typeof procesarTermometro === 'function') procesarTermometro(Number(id));
    }

    function toque(el) {
        var tipo = el.getAttribute('data-tap'), id = Number(el.getAttribute('data-id'));
        if (tipo === 'zona') {
            var a = S.d.areas.find(function (x) { return x.id === id; });
            if (a && a.zona) {
                S.detalle = { id: id, zona: a.zona, nombre: a.nombre };
                S.subzona = null;
                reencuadrar();
                render();
            }
        } else if (tipo === 'punto') {
            abrirRegistro(id);
        }
    }

    function ubicarEnPlano(id) {
        if (!S.d) return;
        var t = S.d.termometros.find(function (x) { return x.id === Number(id); });
        if (!t) return;
        if (t.pos_x == null || t.pos_y == null) {
            abrirRegistro(id);
            return;
        }

        // Si el termómetro está en otra zona, entrar primero a esa zona
        if (!S.detalle || S.detalle.id !== t.area_id) {
            var a = S.d.areas.find(function (x) { return x.id === t.area_id; });
            if (a && a.zona) {
                S.detalle = { id: a.id, zona: a.zona, nombre: a.nombre };
                S.subzona = null;
                reencuadrar();
                render();
            }
        }

        setTimeout(function () {
            var btnPin = $('pin-term-' + t.id) || document.querySelector('button[data-tap="punto"][data-id="' + t.id + '"]');
            if (btnPin) {
                btnPin.classList.add('pin-pulse');
                setTimeout(function () { btnPin.classList.remove('pin-pulse'); }, 4000);
            }
            var marco = $('mapaMarco');
            if (marco && marco.scrollIntoView) {
                marco.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
        }, 50);
    }

    // Pan con 1 dedo + pinch con 2 (Pointer Events); el tap se detecta manual
    // para no disparar tras arrastrar (click fantasma).
    function enlazar() {
        var m = $('mapaMarco');
        m.addEventListener('pointerdown', function (e) {
            // El popup y botones flotantes llevan clicks nativos: sin captura ni pan
            if (e.target && e.target.closest && (e.target.closest('#mapaPopup') || e.target.closest('#mapaBtnFlotanteSalir'))) return;
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
            if (!ptrs.has(e.pointerId)) return;
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
        general: function () {
            S.detalle = null;
            S.subzona = null;
            reencuadrar();
            render();
        },
        seleccionarArea: function (areaId) {
            if (!areaId) {
                S.detalle = null;
                S.subzona = null;
                reencuadrar();
                render();
                return;
            }
            var a = S.d.areas.find(function (x) { return x.id === Number(areaId); });
            if (a && a.zona) {
                S.detalle = { id: a.id, zona: a.zona, nombre: a.nombre };
                S.subzona = null;
                reencuadrar();
                render();
            }
        },
        seleccionarSubzona: function (subzona) {
            S.subzona = subzona || null;
            render();
        },
        toggleFaltantes: function () {
            S.panelFaltantesAbierto = !S.panelFaltantesAbierto;
            renderPanelFaltantes();
        },
        ubicarEnPlano: ubicarEnPlano,
        zoom: function (f) { S.s = Math.min(6, Math.max(0.5, S.s * f)); aplicar(); },
        reset: reencuadrar,
        cerrar: function () { $('mapaPopup').style.display = 'none'; },
        registrar: abrirRegistro
    };
})();
