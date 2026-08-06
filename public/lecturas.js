// ===== Flujo de lecturas ordenadas automáticamente (compartido index/admin) =====
// Manipulación de UI, temas, gauges interactivos y filtros en tiempo real.

// 1. Inicialización y Gestión de Tema (Dark / Light Mode)
function initTheme() {
    const savedTheme = localStorage.getItem('theme') || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
    setTheme(savedTheme);
}

function setTheme(theme) {
    document.documentElement.setAttribute('data-bs-theme', theme);
    document.body.classList.toggle('dark-mode', theme === 'dark');
    localStorage.setItem('theme', theme);
    
    const icon = document.getElementById('themeToggleIcon');
    if (icon) {
        icon.className = theme === 'dark' ? 'bi bi-sun-fill' : 'bi bi-moon-stars-fill';
    }
}

function toggleTheme() {
    const current = localStorage.getItem('theme') === 'dark' ? 'light' : 'dark';
    setTheme(current);
}

document.addEventListener('DOMContentLoaded', () => {
    initTheme();
});

// 2. Stepper de Temperatura (+ / -)
function stepTemp(id, delta) {
    const input = document.getElementById(id);
    if (!input) return;
    const actual = parseFloat(input.value);
    const base = isNaN(actual) ? 0 : actual;
    input.value = (Math.round((base + delta) * 10) / 10).toFixed(1);
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.focus();
}

// 3. Ordenamiento Automático de Lecturas
function ordenarValores(actual, l2, l3) {
    return {
        actual,
        minima: Math.min(actual, l2, l3),
        maxima: Math.max(actual, l2, l3)
    };
}

// 4. Generación de Advertencias para Lecturas
function construirWarningsLecturas(minima, actual, maxima, t) {
    const warnings = [];
    const rangoOperativo = t && t.temp_min_operativa !== undefined && t.temp_max_operativa !== undefined;
    if (rangoOperativo) {
        [['mínima', minima], ['actual', actual], ['máxima', maxima]].forEach(([nombre, v]) => {
            if (v < t.temp_min_operativa || v > t.temp_max_operativa) {
                warnings.push(`<div class="alert alert-warning py-2 small mb-1 d-flex align-items-center gap-2"><i class="bi bi-exclamation-triangle-fill fs-6"></i> <span>La temperatura <strong>${nombre} (${v.toFixed(1)}°C)</strong> está fuera del rango operativo (${t.temp_min_operativa}°C a ${t.temp_max_operativa}°C).</span></div>`);
            }
        });
    }
    if ((maxima - minima) > 15) {
        warnings.push(`<div class="alert alert-warning py-2 small mb-1 d-flex align-items-center gap-2"><i class="bi bi-arrow-down-up fs-6"></i> <span>Hay más de 15°C de diferencia entre la mínima y la máxima. Por favor, verifique que los valores sean correctos.</span></div>`);
    }
    return warnings.join('');
}

// 5. Renderizado de Tarjetas Resumen de Lecturas
function renderTarjetasLecturas(containerId, actual, minima, maxima) {
    const el = document.getElementById(containerId);
    if (!el) return;
    const cards = [
        { cls: 'actual', label: '1º ACTUAL (INSTANTÁNEA)', val: actual, icon: 'bi-thermometer-half' },
        { cls: 'min', label: 'MÍNIMA REGISTRADA', val: minima, icon: 'bi-arrow-down-short' },
        { cls: 'max', label: 'MÁXIMA REGISTRADA', val: maxima, icon: 'bi-arrow-up-short' }
    ].map(c => `
        <div class="col-4">
            <div class="tarjeta-lectura ${c.cls}">
                <div class="tl-label"><i class="bi ${c.icon}"></i> ${c.label}</div>
                <div class="tl-valor">${c.val.toFixed(1)}<span class="tl-unidad">°C</span></div>
            </div>
        </div>`).join('');
    el.innerHTML = cards;
}

// 6. Renderizado de Barra Termómetro Visual (3 Zonas: Óptima / Advertencia / Crítica)
function renderBarra(containerId, minima, actual, maxima, t) {
    const el = document.getElementById(containerId);
    if (!el) return;
    const lo = (t && t.temp_min_fisica !== undefined) ? t.temp_min_fisica : Math.floor(minima - 1);
    const hi = (t && t.temp_max_fisica !== undefined) ? t.temp_max_fisica : Math.ceil(maxima + 1);
    const span = (hi - lo) || 1;
    const pos = v => Math.max(2, Math.min(98, ((v - lo) / span) * 100));

    let zonaVerde = '';
    let zonaAmarillaL = '', zonaAmarillaR = '';
    if (t && t.temp_min_operativa !== undefined && t.temp_max_operativa !== undefined) {
        const minOp = t.temp_min_operativa;
        const maxOp = t.temp_max_operativa;
        const buffer = Math.max(0.5, Math.round(((maxOp - minOp) * 0.15) * 10) / 10);
        
        const zl = Math.max(0, ((minOp - lo) / span) * 100);
        const zr = Math.min(100, ((maxOp - lo) / span) * 100);
        
        const zOptL = Math.max(0, (((minOp + buffer) - lo) / span) * 100);
        const zOptR = Math.min(100, (((maxOp - buffer) - lo) / span) * 100);

        zonaAmarillaL = `<div class="zona-advertencia" style="position:absolute;top:0;bottom:0;left:${zl}%;width:${Math.max(0, zOptL - zl)}%;background:rgba(255,149,0,0.25);" title="Advertencia Preventiva Mínima: ${minOp}°C a ${(minOp+buffer).toFixed(1)}°C"></div>`;
        zonaVerde = `<div class="zona-operativa" style="position:absolute;top:0;bottom:0;left:${zOptL}%;width:${Math.max(0, zOptR - zOptL)}%;background:rgba(52,199,89,0.3);border-left:2px solid #34c759;border-right:2px solid #34c759;" title="Zona Óptima de Laboratorio: ${(minOp+buffer).toFixed(1)}°C a ${(maxOp-buffer).toFixed(1)}°C"></div>`;
        zonaAmarillaR = `<div class="zona-advertencia" style="position:absolute;top:0;bottom:0;left:${zOptR}%;width:${Math.max(0, zr - zOptR)}%;background:rgba(255,149,0,0.25);" title="Advertencia Preventiva Máxima: ${(maxOp-buffer).toFixed(1)}°C a ${maxOp}°C"></div>`;
    }

    const marker = (v, color, label, cls) => `
        <div class="barra-marker ${cls}" style="left:${pos(v)}%;">
            <div class="etiqueta" style="color:${color};">${label}</div>
            <div class="pin" style="background:${color};"></div>
            <div class="valor">${v.toFixed(1)}°</div>
        </div>`;

    el.innerHTML = `
        <div class="barra-termometro">
            ${zonaAmarillaL}
            ${zonaVerde}
            ${zonaAmarillaR}
            ${marker(minima, '#0071e3', 'MIN', 'top')}
            ${marker(actual, '#34c759', 'ACTUAL', 'bottom')}
            ${marker(maxima, '#ff9500', 'MAX', 'top')}
        </div>
        <div class="d-flex justify-content-between small text-muted px-2" style="margin-top:-32px;">
            <span>Límite Mín: ${lo.toFixed(0)}°C</span><span>Límite Máx: ${hi.toFixed(0)}°C</span>
        </div>`;
}

function renderConfirmacionLecturas(containerTarjetas, containerBarra, containerWarnings, confMinId, confActualId, confMaxId, actual, minima, maxima, t) {
    renderTarjetasLecturas(containerTarjetas, actual, minima, maxima);
    renderBarra(containerBarra, minima, actual, maxima, t);
    const warningsEl = document.getElementById(containerWarnings);
    if (warningsEl) warningsEl.innerHTML = construirWarningsLecturas(minima, actual, maxima, t);
    if (confMinId) document.getElementById(confMinId).textContent = minima.toFixed(1);
    if (confActualId) document.getElementById(confActualId).textContent = actual.toFixed(1);
    if (confMaxId) document.getElementById(confMaxId).textContent = maxima.toFixed(1);
}

// 8. Chips Rápidos para Acciones Correctivas Sanitarias (HACCP)
function renderChipsAccionCorrectiva(containerId, textareaId) {
    const container = document.getElementById(containerId);
    if (!container) return;

    const chips = [
        { label: '🚪 Puerta mal cerrada', text: 'Se encontró puerta entornada o mal cerrada. Se cerró correctamente.' },
        { label: '🌡️ Ajuste de termostato', text: 'Se realizó regulación de termostato para estabilizar temperatura.' },
        { label: '⚡ Falla / Corte de energía', text: 'Reporte de interrupción en suministro eléctrico o reinicio de equipo.' },
        { label: '❄️ Ciclo de deshielo', text: 'Equipo se encontraba en ciclo de deshielo automático programado.' },
        { label: '🛠️ Avisado a Mantenimiento', text: 'Se notificó a Mantenimiento para evitar fallas mayores o mermas.' },
        { label: '📦 Carga masiva de producto', text: 'Ingreso reciente de insumos a temperatura ambiente.' }
    ];

    container.innerHTML = `
        <div class="small fw-semibold text-muted mb-1"><i class="bi bi-shield-check me-1 text-primary"></i>Acciones Preventivas / Correctivas Rápidas:</div>
        <div class="apple-chip-group mb-2">
            ${chips.map((c, idx) => `<span class="apple-chip" onclick="insertarChipTexto('${textareaId}', '${c.text}')">${c.label}</span>`).join('')}
        </div>
    `;
}

function insertarChipTexto(textareaId, texto) {
    const txt = document.getElementById(textareaId);
    if (!txt) return;
    if (txt.value.trim().length > 0) {
        txt.value = txt.value.trim() + '. ' + texto;
    } else {
        txt.value = texto;
    }
    txt.dispatchEvent(new Event('input', { bubbles: true }));
}

// 9. Evaluación Visual Sencilla durante el Ingreso (Sin Popups Invasivos)
function evaluarRangoEnTiempoReal(t, actual, minima, maxima, alertContainerId) {
    const alertEl = document.getElementById(alertContainerId);
    if (!alertEl) return;

    if (!t || t.temp_min_operativa === undefined || t.temp_max_operativa === undefined) {
        alertEl.style.display = 'none';
        return;
    }

    const minOp = t.temp_min_operativa;
    const maxOp = t.temp_max_operativa;

    const fueraDeRangoCritico = [];

    const chequearValor = (nombre, val) => {
        if (val === null || isNaN(val)) return;
        if (val < minOp || val > maxOp) {
            fueraDeRangoCritico.push(`${nombre} (${val.toFixed(1)}°C)`);
        }
    };

    chequearValor('Actual', actual);
    chequearValor('Mínima', minima);
    chequearValor('Máxima', maxima);

    if (fueraDeRangoCritico.length > 0) {
        // FUERA DE RANGO OPERATIVO (Umbral Mín / Máx)
        alertEl.style.display = 'block';
        alertEl.className = 'alert alert-danger p-2.5 mb-3 rounded-3 shadow-sm border-danger';
        alertEl.innerHTML = `
            <div class="d-flex align-items-center gap-2 fw-bold text-danger small">
                <i class="bi bi-exclamation-triangle-fill fs-6"></i>
                <span>FUERA DE RANGO OPERATIVO (Límite: <strong>${minOp}°C a ${maxOp}°C</strong>)</span>
            </div>
            <div class="small mt-1 text-danger-emphasis">
                Medición <strong>${fueraDeRangoCritico.join(', ')}</strong> fuera de norma. Se registrará la observación.
            </div>
        `;
    } else if (actual !== null || minima !== null || maxima !== null) {
        // DENTRO DE RANGO OPERATIVO
        alertEl.style.display = 'block';
        alertEl.className = 'alert alert-success p-2 mb-3 rounded-3 shadow-sm border-success';
        alertEl.innerHTML = `
            <div class="d-flex align-items-center gap-2 fw-bold text-success small">
                <i class="bi bi-check-circle-fill fs-6"></i>
                <span>Dentro del rango operativo (${minOp}°C a ${maxOp}°C)</span>
            </div>
        `;
    } else {
        alertEl.style.display = 'none';
    }
}

// 10. Flujo Inteligente de Teclado (Numpad / Enter Automático)
function setupNumpadEnterFlow(modalId) {
    const modalEl = document.getElementById(modalId);
    if (!modalEl) return;

    modalEl.addEventListener('shown.bs.modal', () => {
        const inputActual = document.getElementById('tempActual');
        if (inputActual && !inputActual.disabled) {
            inputActual.focus();
            inputActual.select();
        }
    });

    const fields = ['tempActual', 'lectura2', 'lectura3'];
    fields.forEach((fieldId, index) => {
        const input = document.getElementById(fieldId);
        if (!input) return;

        input.onkeydown = (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const nextFieldId = fields[index + 1];
                if (nextFieldId) {
                    const nextInput = document.getElementById(nextFieldId);
                    if (nextInput) {
                        nextInput.focus();
                        nextInput.select();
                    }
                } else {
                    // Si es el último campo de temperatura, pasar a revisar
                    const btnRevisar = document.getElementById('btnGuardar');
                    if (btnRevisar && btnRevisar.style.display !== 'none') {
                        btnRevisar.click();
                    }
                }
            }
        };
    });
}

// 11. Indicador Intuitivo de Tendencia Térmica (Diferencial de Temperatura)
function calcularBadgeTendencia(tempActual, tempPrevia) {
    if (tempActual === null || tempActual === undefined || tempPrevia === null || tempPrevia === undefined) {
        return '';
    }
    const diff = Math.round((tempActual - tempPrevia) * 10) / 10;
    if (Math.abs(diff) < 0.1) {
        return `<span class="badge bg-light text-secondary border ms-2" title="Temperatura estable frente a la medición anterior"><i class="bi bi-arrow-right me-1"></i>Estable</span>`;
    } else if (diff > 0) {
        return `<span class="badge bg-warning-subtle text-warning border border-warning-subtle ms-2" title="Ascenso de temperatura de +${diff}°C"><i class="bi bi-arrow-up-right me-1"></i>+${diff.toFixed(1)}°C</span>`;
    } else {
        return `<span class="badge bg-info-subtle text-info border border-info-subtle ms-2" title="Descenso de temperatura de ${diff}°C"><i class="bi bi-arrow-down-right me-1"></i>${diff.toFixed(1)}°C</span>`;
    }
}

// 12. Atajos de Teclado Globales (Ctrl+K o / para buscar)
function setupGlobalShortcuts() {
    document.addEventListener('keydown', (e) => {
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
            e.preventDefault();
            const searchInput = document.querySelector('.search-filter-box input, #searchBarGlobal');
            if (searchInput) {
                searchInput.focus();
                searchInput.select();
            }
        }
    });
}

document.addEventListener('DOMContentLoaded', () => {
    setupGlobalShortcuts();
});



