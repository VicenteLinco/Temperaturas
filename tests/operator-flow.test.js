const assert = require('node:assert/strict');
const flow = require('../public/operator-flow.js');

const low = flow.observacionHumedad('Puerta revisada.', 'LOW');
assert.match(low, /\[HUMEDAD:LOW\]/);
assert.equal(flow.estadoHumedad(low), 'LOW');

const error = flow.observacionHumedad(low, 'ERROR');
assert.doesNotMatch(error, /\[HUMEDAD:LOW\]/);
assert.match(error, /\[HUMEDAD:ERROR\]/);
assert.equal(flow.estadoHumedad(error), 'ERROR');
assert.equal(flow.observacionHumedad(error, null), 'Puerta revisada.');

const payloadLow = flow.construirPayloadLectura({
    termometroId: '7', actual: 4, maxima: 6, minima: 2,
    humedad: '', humedadEstado: 'LOW', observaciones: ''
});
assert.equal(payloadLow.humedad, null);
assert.match(payloadLow.observaciones, /\[HUMEDAD:LOW\]/);
const payloadCorregido = flow.construirPayloadLectura({
    termometroId: '7', actual: 4, maxima: 6, minima: 2,
    humedad: '45.2', humedadEstado: null, observaciones: payloadLow.observaciones
});
assert.equal(payloadCorregido.humedad, 45.2);
assert.equal(payloadCorregido.observaciones, null);

const observacionConAccion = `${flow.HUMEDAD_OBSERVACIONES.LOW} · Acción correctiva posterior`;
const payloadConAccion = flow.construirPayloadLectura({
    termometroId: '7', actual: 4, maxima: 6, minima: 2,
    humedad: '44', humedadEstado: null, observaciones: observacionConAccion
});
assert.equal(payloadConAccion.observaciones, 'Acción correctiva posterior');
assert.doesNotMatch(payloadConAccion.observaciones, /\[HUMEDAD:(?:LOW|ERROR)\]/);

const observacionNoCanonica = '[HUMEDAD:LOW] El lector indica LOW · Acción posterior';
const observacionLimpia = flow.observacionHumedad(observacionNoCanonica, null);
assert.equal(observacionLimpia, 'El lector indica LOW · Acción posterior');
assert.equal(flow.estadoHumedad(observacionLimpia), null);

const errorNoCanonico = flow.observacionHumedad('[HUMEDAD:ERROR] El lector indica ERROR · Acción posterior', null);
assert.equal(errorNoCanonico, 'El lector indica ERROR · Acción posterior');
assert.equal(flow.estadoHumedad(errorNoCanonico), null);

const payloadRolesExplicitos = flow.construirPayloadLectura({
    termometroId: '8', actual: 8, maxima: 6, minima: 2,
    humedad: '', humedadEstado: null, observaciones: ''
});
assert.equal(payloadRolesExplicitos.temp_actual, 8);
assert.equal(payloadRolesExplicitos.temp_maxima, 6);
assert.equal(payloadRolesExplicitos.temp_minima, 2);

const areasIncompletas = {
    1: { area: { nombre: 'Microbiología' }, completados: [{ termometro_id: 1 }], pendientes: [] },
    2: { area: { nombre: 'Química' }, completados: [], pendientes: [{ id: 2 }] }
};
assert.deepEqual(flow.totalesRonda(areasIncompletas), { completados: 1, pendientes: 1, total: 2 });
assert.equal(flow.rondaCompleta(areasIncompletas, true), false);

const areasCompletas = {
    1: {
        area: { nombre: 'Microbiología' },
        pendientes: [],
        completados: [{
            termometro_id: 1, termometro_nombre: 'Refrigerador 1', fuera_rango_operativo: false,
            temp_actual: 4, temp_minima: 2, temp_maxima: 6, observaciones: error
        }]
    },
    2: {
        area: { nombre: 'Química' }, pendientes: [],
        completados: [{
            termometro_id: 2, termometro_nombre: 'Congelador 2', fuera_rango_operativo: true,
            temp_actual: -10, temp_minima: -12, temp_maxima: -8, observaciones: null
        }]
    }
};
assert.deepEqual(flow.totalesRonda(areasCompletas), { completados: 2, pendientes: 0, total: 2 });
assert.equal(flow.rondaCompleta(areasCompletas, false), false, 'a partial global load must never complete');
assert.equal(flow.rondaCompleta(areasCompletas, true), true);
const incidencias = flow.recopilarIncidencias(areasCompletas, [{
    id: 3, nombre: 'Sensor 3', area_nombre: 'Química', activo: true, fuera_de_servicio: true
}]);
assert.equal(incidencias.length, 3);
assert.ok(incidencias.some(i => i.tipo === 'Humedad ERROR'));
assert.ok(incidencias.some(i => i.tipo === 'Temperatura fuera de rango'));
assert.ok(incidencias.some(i => i.tipo === 'Fuera de servicio'));

console.log('operator-flow: 28 assertions passed');
