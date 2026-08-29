(function (root, factory) {
    const api = factory();
    if (typeof module === 'object' && module.exports) module.exports = api;
    if (root) root.OperatorFlow = api;
})(typeof globalThis !== 'undefined' ? globalThis : this, function () {
    const HUMEDAD_PREFIX = '[HUMEDAD:';
    const HUMEDAD_OBSERVACIONES = {
        LOW: '[HUMEDAD:LOW] El lector de humedad indica LOW; se registra sin valor de humedad.',
        ERROR: '[HUMEDAD:ERROR] El lector de humedad indica ERROR; se registra sin valor de humedad.'
    };

    function limpiarIncidenciaHumedad(observaciones) {
        let texto = String(observaciones || '');
        Object.values(HUMEDAD_OBSERVACIONES).forEach(canonica => {
            texto = texto.split(canonica).join(' ');
        });
        return texto
            .replace(/\[HUMEDAD:(?:LOW|ERROR)\]/g, ' ')
            .replace(/\s*·\s*·\s*/g, ' · ')
            .replace(/^\s*·\s*|\s*·\s*$/g, '')
            .replace(/\s+/g, ' ')
            .trim();
    }

    function observacionHumedad(observaciones, estado) {
        const base = limpiarIncidenciaHumedad(observaciones);
        const incidencia = HUMEDAD_OBSERVACIONES[estado] || '';
        return [base, incidencia].filter(Boolean).join(' ');
    }

    function estadoHumedad(observaciones) {
        const texto = String(observaciones || '');
        if (texto.includes('[HUMEDAD:LOW]')) return 'LOW';
        if (texto.includes('[HUMEDAD:ERROR]')) return 'ERROR';
        return null;
    }

    function construirPayloadLectura({ termometroId, actual, maxima, minima, humedad, humedadEstado, observaciones }) {
        return {
            termometro_id: Number(termometroId),
            temp_actual: actual,
            temp_maxima: maxima,
            temp_minima: minima,
            humedad: humedadEstado ? null : (String(humedad || '').trim() ? Number(humedad) : null),
            observaciones: observacionHumedad(observaciones, humedadEstado) || null
        };
    }

    function totalesRonda(areasData) {
        return Object.values(areasData || {}).reduce((total, area) => {
            total.completados += (area.completados || []).length;
            total.pendientes += (area.pendientes || []).length;
            total.total = total.completados + total.pendientes;
            return total;
        }, { completados: 0, pendientes: 0, total: 0 });
    }

    function rondaCompleta(areasData, cargaGlobalCompleta) {
        const total = totalesRonda(areasData);
        return Boolean(cargaGlobalCompleta && total.total > 0 && total.pendientes === 0);
    }

    function recopilarIncidencias(areasData, fueraDeServicio) {
        const incidencias = [];
        Object.values(areasData || {}).forEach(area => {
            (area.completados || []).forEach(registro => {
                const humedad = estadoHumedad(registro.observaciones);
                if (!registro.fuera_rango_operativo && !humedad) return;
                const tipos = [];
                const detalles = [];
                if (registro.fuera_rango_operativo) {
                    tipos.push('Temperatura fuera de rango');
                    detalles.push(`actual ${registro.temp_actual ?? '-'}°C, mínima ${registro.temp_minima}°C, máxima ${registro.temp_maxima}°C`);
                }
                if (humedad) {
                    tipos.push(`Humedad ${humedad}`);
                    detalles.push(HUMEDAD_OBSERVACIONES[humedad]);
                }
                incidencias.push({
                    tipo: tipos.join(' + '),
                    area: area.area?.nombre || registro.area_nombre || 'Sin área',
                    equipo: registro.termometro_nombre || `Equipo ${registro.termometro_id}`,
                    detalle: detalles.join('; '),
                    observaciones: registro.observaciones || ''
                });
            });
        });
        (fueraDeServicio || []).filter(equipo => equipo.activo !== false && equipo.fuera_de_servicio).forEach(equipo => {
            incidencias.push({
                tipo: 'Fuera de servicio',
                area: equipo.area_nombre || 'Sin área',
                equipo: equipo.nombre || `Equipo ${equipo.id}`,
                detalle: 'Equipo marcado fuera de servicio',
                observaciones: ''
            });
        });
        return incidencias;
    }

    return {
        HUMEDAD_PREFIX,
        HUMEDAD_OBSERVACIONES,
        limpiarIncidenciaHumedad,
        observacionHumedad,
        estadoHumedad,
        construirPayloadLectura,
        totalesRonda,
        rondaCompleta,
        recopilarIncidencias
    };
});
