const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

// Helpers de shaping multi-serie extraídos de public/admin.html (DOM-free).
// Se evalúan desde el HTML para probar el código real sin cargar el DOM.
const html = fs.readFileSync(path.join(__dirname, '..', 'public', 'admin.html'), 'utf8');

function extraer(nombre) {
    const marca = `function ${nombre}(`;
    const inicio = html.indexOf(marca);
    if (inicio === -1) throw new Error(`${nombre} no existe en admin.html`);
    const abre = html.indexOf('{', inicio);
    let nivel = 0;
    for (let i = abre; i < html.length; i++) {
        if (html[i] === '{') nivel++;
        else if (html[i] === '}') {
            nivel--;
            if (nivel === 0) {
                const src = html.slice(inicio, i + 1);
                return eval(`${src}\n;${nombre};`);
            }
        }
    }
    throw new Error(`cuerpo incompleto para ${nombre}`);
}

const agruparLecturasMultiPorTimestamp = extraer('agruparLecturasMultiPorTimestamp');
const lecturaDePunto = extraer('lecturaDePunto');
const colorSerieTermometro = extraer('colorSerieTermometro');
const alinearTendenciasPorTermometro = extraer('alinearTendenciasPorTermometro');

// Dos termómetros con instantes entrelazados; el índice i NO es el mismo instante.
const lecturas = [
    { termometro_id: 1, termometro_nombre: 'T1', fecha_registro: '2026-10-01 08:00', temp_actual: 4.0, temp_maxima: 5.0, temp_minima: 3.0 },
    { termometro_id: 2, termometro_nombre: 'T2', fecha_registro: '2026-10-01 09:00', temp_actual: 7.0, temp_maxima: 8.0, temp_minima: 6.0 },
    { termometro_id: 1, termometro_nombre: 'T1', fecha_registro: '2026-10-01 10:00', temp_actual: 4.5, temp_maxima: 5.5, temp_minima: 3.5 },
];
const g = agruparLecturasMultiPorTimestamp(lecturas);
assert.deepEqual(g.instantes, ['2026-10-01 08:00', '2026-10-01 09:00', '2026-10-01 10:00']);
assert.equal(g.filas.length, 2);
const f1 = g.filas.find(f => f.id === 1);
const f2 = g.filas.find(f => f.id === 2);
assert.ok(f1.lecturas[0] && f1.lecturas[0].temp_actual === 4.0 && f1.lecturas[1] === null && f1.lecturas[2].temp_actual === 4.5);
assert.ok(f2.lecturas[0] === null && f2.lecturas[1].temp_actual === 7.0 && f2.lecturas[2] === null);
assert.ok(agruparLecturasMultiPorTimestamp(null).instantes.length === 0 && agruparLecturasMultiPorTimestamp(null).filas.length === 0);

// Resolución por dataset, nunca lecturas[globalIndex].
const ds = { label: 'T2 (Actual)', metaLecturas: f2.lecturas };
assert.equal(lecturaDePunto(ds, 1, lecturas), f2.lecturas[1]);
assert.equal(lecturaDePunto(ds, 0, lecturas), null);
assert.equal(lecturaDePunto({ label: 'x' }, 2, lecturas), lecturas[2]);
assert.equal(lecturaDePunto(null, 0, null), null);

// Color distinto por termómetro.
const c0 = colorSerieTermometro(0), c1 = colorSerieTermometro(1), c2 = colorSerieTermometro(2);
assert.ok(c0 !== c1 && c1 !== c2 && c0 !== c2);
assert.ok(typeof colorSerieTermometro(8) === 'string' && typeof colorSerieTermometro(100) === 'string');

// Desglose diario por termómetro alineado por fecha.
const detalle = [
    { termometro_id: 1, termometro_nombre: 'T1', fecha: '2026-10-01', promedio_actual: 4.0, promedio_max: 5.0, promedio_min: 3.0, alertas: 0, total_registros: 2 },
    { termometro_id: 1, termometro_nombre: 'T1', fecha: '2026-10-02', promedio_actual: 4.2, promedio_max: 5.2, promedio_min: 3.2, alertas: 1, total_registros: 2 },
    { termometro_id: 2, termometro_nombre: 'T2', fecha: '2026-10-02', promedio_actual: 7.1, promedio_max: 8.1, promedio_min: 6.1, alertas: 0, total_registros: 1 },
];
const a = alinearTendenciasPorTermometro(detalle);
assert.deepEqual(a.fechas, ['2026-10-01', '2026-10-02']);
const a1 = a.filas.find(f => f.id === 1);
const a2 = a.filas.find(f => f.id === 2);
assert.ok(a1.puntos[0].promedio_actual === 4.0 && a1.puntos[1].promedio_actual === 4.2);
assert.ok(a2.puntos[0] === null && a2.puntos[1].promedio_actual === 7.1);

console.log('graficos-multi-series: 14 assertions passed');
