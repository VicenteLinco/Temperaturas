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

// 6. Renderizado de Barra Termómetro Visual (Mín / Actual / Máx / Rango Operativo)
function renderBarra(containerId, minima, actual, maxima, t) {
    const el = document.getElementById(containerId);
    if (!el) return;
    const lo = (t && t.temp_min_fisica !== undefined) ? t.temp_min_fisica : Math.floor(minima - 1);
    const hi = (t && t.temp_max_fisica !== undefined) ? t.temp_max_fisica : Math.ceil(maxima + 1);
    const span = (hi - lo) || 1;
    const pos = v => Math.max(2, Math.min(98, ((v - lo) / span) * 100));

    let zona = '';
    if (t && t.temp_min_operativa !== undefined && t.temp_max_operativa !== undefined) {
        const zl = Math.max(0, ((t.temp_min_operativa - lo) / span) * 100);
        const zr = Math.min(100, ((t.temp_max_operativa - lo) / span) * 100);
        zona = `<div class="zona-operativa" style="left:${zl}%;width:${Math.max(0, zr - zl)}%;" title="Rango Operativo Seguro: ${t.temp_min_operativa}°C a ${t.temp_max_operativa}°C"></div>`;
    }

    const marker = (v, color, label, cls) => `
        <div class="barra-marker ${cls}" style="left:${pos(v)}%;">
            <div class="etiqueta" style="color:${color};">${label}</div>
            <div class="pin" style="background:${color};"></div>
            <div class="valor">${v.toFixed(1)}°</div>
        </div>`;

    el.innerHTML = `
        <div class="barra-termometro">
            ${zona}
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

// 7. Filtro Rápido en Tiempo Real para Tablas o Listas de Tarjetas
function setupLiveFilter(inputId, itemsContainerSelector, itemCardSelector, searchFields) {
    const input = document.getElementById(inputId);
    if (!input) return;

    input.addEventListener('input', (e) => {
        const query = e.target.value.toLowerCase().trim();
        const container = document.querySelector(itemsContainerSelector);
        if (!container) return;

        const items = container.querySelectorAll(itemCardSelector);
        items.forEach(item => {
            const textToSearch = searchFields.map(attr => {
                if (attr.startsWith('.')) return (item.querySelector(attr)?.textContent || '');
                return item.getAttribute(attr) || item.textContent || '';
            }).join(' ').toLowerCase();

            if (textToSearch.includes(query)) {
                item.style.display = '';
            } else {
                item.style.display = 'none';
            }
        });
    });
}
