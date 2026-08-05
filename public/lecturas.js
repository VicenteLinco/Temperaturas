// ===== Flujo de lecturas ordenadas automáticamente (compartido index/admin) =====
// Estas funciones no dependen de la UI específica: solo manipulan inputs y contenedores.

function stepTemp(id, delta) {
    const input = document.getElementById(id);
    const actual = parseFloat(input.value);
    const base = isNaN(actual) ? 0 : actual;
    input.value = Math.round((base + delta) * 10) / 10;
    input.focus();
}

function ordenarValores(actual, l2, l3) {
    return {
        actual,
        minima: Math.min(actual, l2, l3),
        maxima: Math.max(actual, l2, l3)
    };
}

function construirWarningsLecturas(minima, actual, maxima, t) {
    const warnings = [];
    const rangoOperativo = t && t.temp_min_operativa !== undefined && t.temp_max_operativa !== undefined;
    if (rangoOperativo) {
        [['mínima', minima], ['actual', actual], ['máxima', maxima]].forEach(([nombre, v]) => {
            if (v < t.temp_min_operativa || v > t.temp_max_operativa) {
                warnings.push(`<div class="alert alert-warning py-2 small mb-1"><i class="bi bi-exclamation-triangle"></i> La temperatura <strong>${nombre} (${v.toFixed(1)}°C)</strong> está fuera del rango operativo (${t.temp_min_operativa}°C a ${t.temp_max_operativa}°C).</div>`);
            }
        });
    }
    if ((maxima - minima) > 15) {
        warnings.push(`<div class="alert alert-warning py-2 small mb-1"><i class="bi bi-arrow-down-up"></i> Hay más de 15°C entre la mínima y la máxima. Verifica que las lecturas sean correctas.</div>`);
    }
    return warnings.join('');
}

function renderTarjetasLecturas(containerId, actual, minima, maxima) {
    const cards = [
        { cls: 'actual', label: '1º ACTUAL (PRINCIPAL)', val: actual },
        { cls: 'min', label: 'MÍNIMA', val: minima },
        { cls: 'max', label: 'MÁXIMA', val: maxima }
    ].map(c => `
        <div class="col-4">
            <div class="tarjeta-lectura ${c.cls}">
                <div class="tl-label">${c.label}</div>
                <div class="tl-valor">${c.val.toFixed(1)}<span class="tl-unidad">°</span></div>
            </div>
        </div>`).join('');
    document.getElementById(containerId).innerHTML = cards;
}

function renderBarra(containerId, minima, actual, maxima, t) {
    const lo = (t && t.temp_min_fisica !== undefined) ? t.temp_min_fisica : Math.floor(minima - 1);
    const hi = (t && t.temp_max_fisica !== undefined) ? t.temp_max_fisica : Math.ceil(maxima + 1);
    const span = (hi - lo) || 1;
    const pos = v => Math.max(2, Math.min(98, ((v - lo) / span) * 100));

    let zona = '';
    if (t && t.temp_min_operativa !== undefined && t.temp_max_operativa !== undefined) {
        const zl = Math.max(0, ((t.temp_min_operativa - lo) / span) * 100);
        const zr = Math.min(100, ((t.temp_max_operativa - lo) / span) * 100);
        zona = `<div class="zona-operativa" style="left:${zl}%;width:${Math.max(0, zr - zl)}%;"></div>`;
    }

    const marker = (v, color, label, cls) => `
        <div class="barra-marker ${cls}" style="left:${pos(v)}%;">
            <div class="etiqueta" style="color:${color};">${label}</div>
            <div class="pin" style="background:${color};"></div>
            <div class="valor">${v.toFixed(1)}°</div>
        </div>`;

    document.getElementById(containerId).innerHTML = `
        <div class="barra-termometro">
            ${zona}
            ${marker(minima, '#2563eb', 'MIN', 'top')}
            ${marker(actual, '#16a34a', 'ACTUAL', 'bottom')}
            ${marker(maxima, '#dc2626', 'MAX', 'top')}
        </div>
        <div class="d-flex justify-content-between small text-muted px-2" style="margin-top:-32px;">
            <span>${lo.toFixed(0)}°</span><span>${hi.toFixed(0)}°</span>
        </div>`;
}

function renderConfirmacionLecturas(containerTarjetas, containerBarra, containerWarnings, confMinId, confActualId, confMaxId, actual, minima, maxima, t) {
    renderTarjetasLecturas(containerTarjetas, actual, minima, maxima);
    renderBarra(containerBarra, minima, actual, maxima, t);
    document.getElementById(containerWarnings).innerHTML = construirWarningsLecturas(minima, actual, maxima, t);
    if (confMinId) document.getElementById(confMinId).textContent = minima.toFixed(1);
    if (confActualId) document.getElementById(confActualId).textContent = actual.toFixed(1);
    if (confMaxId) document.getElementById(confMaxId).textContent = maxima.toFixed(1);
}
