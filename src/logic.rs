use crate::models::TipoTermometro;
use anyhow::{anyhow, Result};
use chrono::{Local, NaiveDate, NaiveTime, Timelike};

/// Representa una ventana horaria de registro
#[derive(Debug, Clone)]
pub struct VentanaHoraria {
    pub nombre: String, // "14:00" o "02:00"
    #[allow(dead_code)]
    pub hora_central: NaiveTime,
    #[allow(dead_code)]
    pub hora_inicio: NaiveTime,
    #[allow(dead_code)]
    pub hora_fin: NaiveTime,
    #[allow(dead_code)]
    pub es_turno_noche: bool, // true si es el turno 20pm-8am
}

/// Helper para verificar si una hora está dentro de un rango con tolerancia (en minutos)
/// Maneja cruce de medianoche
fn esta_en_rango(hora_central: NaiveTime, tolerancia_minutos: i32, ahora: NaiveTime) -> bool {
    let minutos_central = hora_central.num_seconds_from_midnight() as i64 / 60;
    let minutos_ahora = ahora.num_seconds_from_midnight() as i64 / 60;

    // Normalizar a un día de 1440 minutos
    let diff = (minutos_ahora - minutos_central).abs();

    // Caso directo: diferencia menor a tolerancia
    if diff <= tolerancia_minutos as i64 {
        return true;
    }

    // Caso cruce medianoche: (ej: central 02:00 (120m), ahora 23:00 (1380m))
    let diff_circular = 1440 - diff;
    if diff_circular <= tolerancia_minutos as i64 {
        return true;
    }

    false
}

/// Determina la ventana horaria actual basándose en la hora de Chile
pub fn determinar_ventana_actual(
    hora_1: &str, // "14:00" turno día
    hora_2: &str, // "02:00" turno noche
    tolerancia_minutos: i32,
    restriccion_activa: bool,
) -> Result<Option<VentanaHoraria>> {
    // Obtener hora actual en Chile
    let ahora_chile = Local::now().with_timezone(&chrono_tz::America::Santiago);
    let ahora = ahora_chile.time();

    let hora_central_1 = NaiveTime::parse_from_str(hora_1, "%H:%M")?;
    let hora_central_2 = NaiveTime::parse_from_str(hora_2, "%H:%M")?;

    if restriccion_activa {
        if esta_en_rango(hora_central_1, tolerancia_minutos, ahora) {
            let hora_inicio_dia = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
            let hora_fin_dia = NaiveTime::from_hms_opt(20, 0, 0).unwrap();

            Ok(Some(VentanaHoraria {
                nombre: hora_1.to_string(),
                hora_central: hora_central_1,
                hora_inicio: hora_inicio_dia,
                hora_fin: hora_fin_dia,
                es_turno_noche: false,
            }))
        } else if esta_en_rango(hora_central_2, tolerancia_minutos, ahora) {
            let hora_inicio_dia = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
            let hora_fin_dia = NaiveTime::from_hms_opt(20, 0, 0).unwrap();

            Ok(Some(VentanaHoraria {
                nombre: hora_2.to_string(),
                hora_central: hora_central_2,
                hora_inicio: hora_fin_dia,
                hora_fin: hora_inicio_dia,
                es_turno_noche: true,
            }))
        } else {
            Ok(None)
        }
    } else {
        let hora_inicio_dia = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let hora_fin_dia = NaiveTime::from_hms_opt(20, 0, 0).unwrap();

        if ahora >= hora_inicio_dia && ahora < hora_fin_dia {
            Ok(Some(VentanaHoraria {
                nombre: hora_1.to_string(),
                hora_central: hora_central_1,
                hora_inicio: hora_inicio_dia,
                hora_fin: hora_fin_dia,
                es_turno_noche: false,
            }))
        } else {
            Ok(Some(VentanaHoraria {
                nombre: hora_2.to_string(),
                hora_central: hora_central_2,
                hora_inicio: hora_fin_dia,
                hora_fin: hora_inicio_dia,
                es_turno_noche: true,
            }))
        }
    }
}

/// Estado de la ventana de registro en un instante dado.
///
/// Con la restricción activa solo se puede registrar dentro de la ventana. Cuando
/// está cerrada hace falta saber a qué hora vuelve a abrir, para poder decírselo al
/// operador en vez de dejarlo con un error sin explicación.
#[derive(Debug, Clone)]
pub struct EstadoVentana {
    /// Ventana vigente; si ninguna lo está, la última que se cerró.
    pub ventana: VentanaHoraria,
    pub activa: bool,
    /// Hora a la que abre la próxima ventana (solo tiene sentido si `activa` es falso).
    pub proxima_apertura: NaiveTime,
}

fn minutos_de(t: NaiveTime) -> i64 {
    t.num_seconds_from_midnight() as i64 / 60
}

/// Minutos que faltan desde `desde` hasta `hasta` avanzando en el reloj (0..1439).
fn minutos_hacia_adelante(desde: i64, hasta: i64) -> i64 {
    ((hasta - desde) % 1440 + 1440) % 1440
}

/// Calcula el estado de la ventana en la hora local de Chile.
///
/// Sin restricción se conserva el comportamiento histórico (siempre hay ventana, con
/// el corte fijo 08:00/20:00) para no alterar instalaciones que no la usan.
pub fn estado_ventana_actual(
    hora_1: &str,
    hora_2: &str,
    tolerancia_minutos: i32,
    restriccion_activa: bool,
) -> Result<EstadoVentana> {
    let ahora = Local::now()
        .with_timezone(&chrono_tz::America::Santiago)
        .time();
    estado_ventana_en(
        hora_1,
        hora_2,
        tolerancia_minutos,
        restriccion_activa,
        ahora,
    )
}

/// Igual que `estado_ventana_actual` pero con la hora inyectada, para poder probarla.
pub fn estado_ventana_en(
    hora_1: &str,
    hora_2: &str,
    tolerancia_minutos: i32,
    restriccion_activa: bool,
    ahora: NaiveTime,
) -> Result<EstadoVentana> {
    let central_1 = NaiveTime::parse_from_str(hora_1, "%H:%M")?;
    let central_2 = NaiveTime::parse_from_str(hora_2, "%H:%M")?;

    let inicio_dia = NaiveTime::from_hms_opt(8, 0, 0).ok_or_else(|| anyhow!("hora inválida"))?;
    let fin_dia = NaiveTime::from_hms_opt(20, 0, 0).ok_or_else(|| anyhow!("hora inválida"))?;

    let construir = |nombre: &str, central: NaiveTime, noche: bool| VentanaHoraria {
        nombre: nombre.to_string(),
        hora_central: central,
        hora_inicio: if noche { fin_dia } else { inicio_dia },
        hora_fin: if noche { inicio_dia } else { fin_dia },
        es_turno_noche: noche,
    };

    if !restriccion_activa {
        let de_dia = ahora >= inicio_dia && ahora < fin_dia;
        let ventana = if de_dia {
            construir(hora_1, central_1, false)
        } else {
            construir(hora_2, central_2, true)
        };
        return Ok(EstadoVentana {
            ventana,
            activa: true,
            proxima_apertura: ahora,
        });
    }

    let candidatas = [(hora_1, central_1, false), (hora_2, central_2, true)];

    // Ventana vigente
    for (nombre, central, noche) in candidatas.iter() {
        if esta_en_rango(*central, tolerancia_minutos, ahora) {
            return Ok(EstadoVentana {
                ventana: construir(nombre, *central, *noche),
                activa: true,
                proxima_apertura: ahora,
            });
        }
    }

    // Cerrada: se busca la que abre antes (para avisar) y la que cerró hace menos
    // (para seguir mostrando la ronda recién terminada en vez de una pantalla vacía).
    let ahora_min = minutos_de(ahora);
    let tol = tolerancia_minutos as i64;

    let mut proxima = None::<i64>;
    let mut ultima: Option<(i64, &str, NaiveTime, bool)> = None;

    for (nombre, central, noche) in candidatas.iter() {
        let central_min = minutos_de(*central);
        let apertura = ((central_min - tol) % 1440 + 1440) % 1440;
        let cierre = ((central_min + tol) % 1440 + 1440) % 1440;

        let falta = minutos_hacia_adelante(ahora_min, apertura);
        if proxima.map_or(true, |p| falta < minutos_hacia_adelante(ahora_min, p)) {
            proxima = Some(apertura);
        }

        let desde_cierre = minutos_hacia_adelante(cierre, ahora_min);
        if ultima
            .as_ref()
            .map_or(true, |(d, _, _, _)| desde_cierre < *d)
        {
            ultima = Some((desde_cierre, nombre, *central, *noche));
        }
    }

    let (_, nombre, central, noche) = ultima.ok_or_else(|| anyhow!("sin ventanas configuradas"))?;
    let apertura = proxima.unwrap_or(0);

    Ok(EstadoVentana {
        ventana: construir(nombre, central, noche),
        activa: false,
        proxima_apertura: NaiveTime::from_hms_opt(
            (apertura / 60) as u32,
            (apertura % 60) as u32,
            0,
        )
        .ok_or_else(|| anyhow!("hora de apertura inválida"))?,
    })
}

/// Calcula el día asignado para un registro basándose en el turno
#[allow(dead_code)]
pub fn calcular_dia_asignado(
    ventana: &VentanaHoraria,
    fecha_registro: &chrono::NaiveDateTime,
) -> NaiveDate {
    let fecha_actual = fecha_registro.date();

    if ventana.es_turno_noche {
        let hora_registro = fecha_registro.time();
        let hora_corte = NaiveTime::from_hms_opt(20, 0, 0).unwrap();

        if hora_registro >= hora_corte {
            fecha_actual + chrono::Duration::days(1)
        } else {
            fecha_actual
        }
    } else {
        fecha_actual
    }
}

/// Resultado de validación de temperatura/humedad
#[derive(Debug)]
pub enum ValidacionResultado {
    Ok,
    Advertencia(String),
    Rechazo(String),
}

/// Valida una medición de temperatura
pub fn validar_temperatura(
    temp: f32,
    tipo: &TipoTermometro,
    campo: &str, // "máxima" o "mínima"
) -> ValidacionResultado {
    if temp < tipo.temp_min_fisica || temp > tipo.temp_max_fisica {
        return ValidacionResultado::Rechazo(format!(
            "Temperatura {} ({:.1}°C) fuera de rango físico ({:.1}°C a {:.1}°C)",
            campo, temp, tipo.temp_min_fisica, tipo.temp_max_fisica
        ));
    }

    if temp < tipo.temp_min_operativa || temp > tipo.temp_max_operativa {
        return ValidacionResultado::Advertencia(format!(
            "Temperatura {} ({:.1}°C) fuera de rango operativo ({:.1}°C a {:.1}°C)",
            campo, temp, tipo.temp_min_operativa, tipo.temp_max_operativa
        ));
    }

    ValidacionResultado::Ok
}

/// Valida una medición de humedad
pub fn validar_humedad(humedad: f32, tipo: &TipoTermometro) -> Result<ValidacionResultado> {
    if !tipo.tiene_humedad {
        return Err(anyhow!("Este tipo de termómetro no mide humedad"));
    }

    let hum_min_fisica = tipo
        .hum_min_fisica
        .ok_or(anyhow!("Rango físico de humedad no configurado"))?;
    let hum_max_fisica = tipo
        .hum_max_fisica
        .ok_or(anyhow!("Rango físico de humedad no configurado"))?;
    let hum_min_operativa = tipo
        .hum_min_operativa
        .ok_or(anyhow!("Rango operativo de humedad no configurado"))?;
    let hum_max_operativa = tipo
        .hum_max_operativa
        .ok_or(anyhow!("Rango operativo de humedad no configurado"))?;

    if humedad < hum_min_fisica || humedad > hum_max_fisica {
        return Ok(ValidacionResultado::Rechazo(format!(
            "Humedad ({:.1}%) fuera de rango físico ({:.1}% a {:.1}%)",
            humedad, hum_min_fisica, hum_max_fisica
        )));
    }

    if humedad < hum_min_operativa || humedad > hum_max_operativa {
        return Ok(ValidacionResultado::Advertencia(format!(
            "Humedad ({:.1}%) fuera de rango operativo ({:.1}% a {:.1}%)",
            humedad, hum_min_operativa, hum_max_operativa
        )));
    }

    Ok(ValidacionResultado::Ok)
}

/// LOW/ERROR is persisted in observations so a missing numeric humidity is
/// explicit, auditable, and can be included in the incident summary.
pub fn observacion_reporta_humedad_no_disponible(observaciones: Option<&str>) -> bool {
    observaciones
        .is_some_and(|texto| texto.contains("[HUMEDAD:LOW]") || texto.contains("[HUMEDAD:ERROR]"))
}

/// Resolves PATCH/PUT-style nullable fields: omitted keeps the persisted value,
/// while an explicit JSON null clears it.
pub fn resolver_campo_nullable<T>(nuevo: Option<Option<T>>, actual: Option<T>) -> Option<T> {
    nuevo.unwrap_or(actual)
}

/// Valida un registro completo
pub fn validar_registro(
    temp_actual: Option<f32>,
    temp_maxima: f32,
    temp_minima: f32,
    humedad: Option<f32>,
    humedad_no_disponible_reportada: bool,
    tipo: &TipoTermometro,
) -> Result<(bool, Vec<String>)> {
    let mut advertencias = Vec::new();
    let mut fuera_rango_operativo = false;

    if temp_maxima < temp_minima {
        return Err(anyhow!(
            "Temperatura máxima ({:.1}°C) no puede ser menor que temperatura mínima ({:.1}°C)",
            temp_maxima,
            temp_minima
        ));
    }

    if let Some(actual) = temp_actual {
        match validar_temperatura(actual, tipo, "actual") {
            ValidacionResultado::Ok => {}
            ValidacionResultado::Advertencia(msg) => {
                advertencias.push(msg);
                fuera_rango_operativo = true;
            }
            ValidacionResultado::Rechazo(msg) => return Err(anyhow!(msg)),
        }
    }

    match validar_temperatura(temp_maxima, tipo, "máxima") {
        ValidacionResultado::Ok => {}
        ValidacionResultado::Advertencia(msg) => {
            advertencias.push(msg);
            fuera_rango_operativo = true;
        }
        ValidacionResultado::Rechazo(msg) => {
            return Err(anyhow!(msg));
        }
    }

    match validar_temperatura(temp_minima, tipo, "mínima") {
        ValidacionResultado::Ok => {}
        ValidacionResultado::Advertencia(msg) => {
            advertencias.push(msg);
            fuera_rango_operativo = true;
        }
        ValidacionResultado::Rechazo(msg) => {
            return Err(anyhow!(msg));
        }
    }

    if let Some(h) = humedad {
        match validar_humedad(h, tipo)? {
            ValidacionResultado::Ok => {}
            ValidacionResultado::Advertencia(msg) => {
                advertencias.push(msg);
                fuera_rango_operativo = true;
            }
            ValidacionResultado::Rechazo(msg) => {
                return Err(anyhow!(msg));
            }
        }
    } else if tipo.tiene_humedad && !humedad_no_disponible_reportada {
        return Err(anyhow!(
            "Este tipo de termómetro requiere medición de humedad"
        ));
    }

    Ok((fuera_rango_operativo, advertencias))
}

/// Ordena exclusivamente las lecturas explícitas de máxima y mínima.
/// La temperatura actual conserva siempre su rol y nunca reemplaza un extremo.
pub fn normalizar_lecturas(
    temp_actual: Option<f32>,
    lectura_a: f32,
    lectura_b: f32,
) -> (Option<f32>, f32, f32) {
    let temp_minima = lectura_a.min(lectura_b);
    let temp_maxima = lectura_a.max(lectura_b);

    (temp_actual, temp_maxima, temp_minima)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    fn tipo_base() -> TipoTermometro {
        TipoTermometro {
            id: 1,
            nombre: "Refrigerador".to_string(),
            descripcion: None,
            tiene_humedad: false,
            temp_min_operativa: 2.0,
            temp_max_operativa: 8.0,
            temp_min_fisica: -10.0,
            temp_max_fisica: 30.0,
            hum_min_operativa: None,
            hum_max_operativa: None,
            hum_min_fisica: None,
            hum_max_fisica: None,
            activo: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn tipo_con_humedad() -> TipoTermometro {
        let mut t = tipo_base();
        t.tiene_humedad = true;
        t.hum_min_operativa = Some(30.0);
        t.hum_max_operativa = Some(60.0);
        t.hum_min_fisica = Some(0.0);
        t.hum_max_fisica = Some(100.0);
        t
    }

    #[test]
    fn rango_directo_dentro_de_tolerancia() {
        let central = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let ahora = NaiveTime::from_hms_opt(15, 0, 0).unwrap();
        assert!(esta_en_rango(central, 119, ahora));
    }

    #[test]
    fn rango_directo_fuera_de_tolerancia() {
        let central = NaiveTime::from_hms_opt(14, 0, 0).unwrap();
        let ahora = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
        assert!(!esta_en_rango(central, 119, ahora));
    }

    #[test]
    fn rango_cruce_medianoche() {
        // Central 02:00, ahora 23:30 → distancia circular 150 min, dentro de 180
        let central = NaiveTime::from_hms_opt(2, 0, 0).unwrap();
        let ahora = NaiveTime::from_hms_opt(23, 30, 0).unwrap();
        assert!(esta_en_rango(central, 180, ahora));
        // Ahora 21:00 → distancia circular 300 min, fuera
        let fuera = NaiveTime::from_hms_opt(21, 0, 0).unwrap();
        assert!(!esta_en_rango(central, 180, fuera));
    }

    #[test]
    fn validar_temperatura_normal() {
        let tipo = tipo_base();
        assert!(matches!(
            validar_temperatura(5.0, &tipo, "máxima"),
            ValidacionResultado::Ok
        ));
    }

    #[test]
    fn validar_temperatura_advertencia() {
        let tipo = tipo_base();
        assert!(matches!(
            validar_temperatura(20.0, &tipo, "máxima"),
            ValidacionResultado::Advertencia(_)
        ));
    }

    #[test]
    fn validar_temperatura_rechazo_fisico() {
        let tipo = tipo_base();
        assert!(matches!(
            validar_temperatura(-50.0, &tipo, "mínima"),
            ValidacionResultado::Rechazo(_)
        ));
    }

    #[test]
    fn validar_humedad_sin_capacidad_da_error() {
        let tipo = tipo_base();
        assert!(validar_humedad(50.0, &tipo).is_err());
    }

    #[test]
    fn validar_humedad_advertencia() {
        let tipo = tipo_con_humedad();
        let res = validar_humedad(70.0, &tipo).unwrap();
        assert!(matches!(res, ValidacionResultado::Advertencia(_)));
    }

    #[test]
    fn validar_registro_max_menor_que_min_error() {
        let tipo = tipo_base();
        let res = validar_registro(None, 3.0, 5.0, None, false, &tipo);
        assert!(res.is_err());
    }

    #[test]
    fn validar_registro_ok() {
        let tipo = tipo_base();
        let (fuera, advertencias) = validar_registro(None, 6.0, 4.0, None, false, &tipo).unwrap();
        assert!(!fuera);
        assert!(advertencias.is_empty());
    }

    #[test]
    fn validar_registro_humedad_requerida() {
        let tipo = tipo_con_humedad();
        assert!(validar_registro(None, 6.0, 4.0, None, false, &tipo).is_err());
    }

    #[test]
    fn validar_registro_acepta_humedad_low_o_error_reportada() {
        let tipo = tipo_con_humedad();
        let (fuera, advertencias) = validar_registro(None, 6.0, 4.0, None, true, &tipo).unwrap();
        assert!(!fuera);
        assert!(advertencias.is_empty());
    }

    #[test]
    fn reconoce_marcadores_tecnicos_de_humedad() {
        assert!(observacion_reporta_humedad_no_disponible(Some(
            "[HUMEDAD:LOW] El lector indica LOW"
        )));
        assert!(observacion_reporta_humedad_no_disponible(Some(
            "[HUMEDAD:ERROR] El lector indica ERROR"
        )));
        assert!(!observacion_reporta_humedad_no_disponible(Some(
            "Humedad no anotada"
        )));
    }

    #[test]
    fn actualizar_registro_distingue_humedad_nula_de_campo_omitido() {
        let explicita: crate::models::ActualizarRegistroRequest =
            serde_json::from_str(r#"{"humedad":null,"observaciones":"[HUMEDAD:LOW] Sin lectura"}"#)
                .unwrap();
        assert_eq!(explicita.humedad, Some(None));

        let omitida: crate::models::ActualizarRegistroRequest =
            serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(omitida.humedad, None);

        let humedad_actual = Some(45.0_f32);
        assert_eq!(
            resolver_campo_nullable(omitida.humedad, humedad_actual),
            Some(45.0)
        );
        assert_eq!(
            resolver_campo_nullable(explicita.humedad, humedad_actual),
            None
        );
        let observaciones = explicita.observaciones.flatten();
        assert!(observacion_reporta_humedad_no_disponible(
            observaciones.as_deref()
        ));
    }

    #[test]
    fn validar_registro_ok_con_humedad() {
        let tipo = tipo_con_humedad();
        let (fuera, advertencias) =
            validar_registro(None, 6.0, 4.0, Some(50.0), false, &tipo).unwrap();
        assert!(!fuera);
        assert!(advertencias.is_empty());
    }

    #[test]
    fn validar_registro_marca_fuera_rango() {
        let tipo = tipo_base();
        let (fuera, advertencias) = validar_registro(None, 20.0, 4.0, None, false, &tipo).unwrap();
        assert!(fuera);
        assert!(!advertencias.is_empty());
    }

    #[test]
    fn validar_registro_marca_actual_fuera_de_rango_operativo() {
        let tipo = tipo_base();
        let (fuera, advertencias) =
            validar_registro(Some(20.0), 6.0, 4.0, None, false, &tipo).unwrap();
        assert!(fuera);
        assert!(advertencias.iter().any(|msg| msg.contains("actual")));
    }

    #[test]
    fn validar_registro_rechaza_actual_fisicamente_imposible() {
        let tipo = tipo_base();
        let error = validar_registro(Some(-50.0), 6.0, 4.0, None, false, &tipo).unwrap_err();
        assert!(error.to_string().contains("actual"));
    }

    #[test]
    fn normalizar_lecturas_ordena_correctamente_con_negativos() {
        // El registrador ingresó las lecturas al revés: -12 como "máxima" y -18 como "mínima"
        let (actual, maxima, minima) = normalizar_lecturas(Some(-15.0), -12.0, -18.0);
        assert_eq!(actual, Some(-15.0));
        assert_eq!(maxima, -12.0);
        assert_eq!(minima, -18.0);
    }

    #[test]
    fn normalizar_lecturas_preserva_actual_fuera_de_extremos() {
        let (actual, maxima, minima) = normalizar_lecturas(Some(-25.0), -12.0, -18.0);
        assert_eq!(actual, Some(-25.0));
        assert_eq!(maxima, -12.0);
        assert_eq!(minima, -18.0);
    }

    #[test]
    fn normalizar_lecturas_sin_actual() {
        let (actual, maxima, minima) = normalizar_lecturas(None, -12.0, -18.0);
        assert_eq!(actual, None);
        assert_eq!(maxima, -12.0);
        assert_eq!(minima, -18.0);
    }

    // ===== VENTANAS HORARIAS =====
    // Configuración real del usuario: rondas de 14:00 y 02:00.

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn estado(hora: NaiveTime, tol: i32) -> EstadoVentana {
        estado_ventana_en("14:00", "02:00", tol, true, hora).unwrap()
    }

    #[test]
    fn dentro_de_la_ventana_de_las_14_se_registra_en_esa_ronda() {
        for hora in [t(12, 0), t(13, 30), t(14, 0), t(15, 59), t(16, 0)] {
            let e = estado(hora, 120);
            assert!(e.activa, "{:?} debería estar dentro de la ventana", hora);
            assert_eq!(e.ventana.nombre, "14:00");
        }
    }

    #[test]
    fn dentro_de_la_ventana_de_las_02_se_registra_en_esa_ronda() {
        for hora in [t(0, 0), t(1, 15), t(2, 0), t(3, 59), t(4, 0)] {
            let e = estado(hora, 120);
            assert!(e.activa, "{:?} debería estar dentro de la ventana", hora);
            assert_eq!(e.ventana.nombre, "02:00");
        }
    }

    #[test]
    fn fuera_de_ventana_no_se_puede_registrar() {
        for hora in [t(4, 30), t(9, 0), t(11, 59), t(17, 0), t(20, 0), t(23, 59)] {
            let e = estado(hora, 120);
            assert!(!e.activa, "{:?} NO debería permitir registrar", hora);
        }
    }

    #[test]
    fn fuera_de_ventana_informa_cuando_abre_la_proxima() {
        // A las 17:00 la siguiente en abrir es la de las 02:00 (abre a medianoche)
        assert_eq!(estado(t(17, 0), 120).proxima_apertura, t(0, 0));
        // A las 09:00 la siguiente es la de las 14:00 (abre a las 12:00)
        assert_eq!(estado(t(9, 0), 120).proxima_apertura, t(12, 0));
        // Justo tras cerrar la de las 02:00, la próxima es la de las 14:00
        assert_eq!(estado(t(4, 30), 120).proxima_apertura, t(12, 0));
    }

    #[test]
    fn fuera_de_ventana_conserva_la_ronda_recien_cerrada() {
        // A las 17:00 acaba de cerrar la de las 14:00: es la que se sigue mostrando
        assert_eq!(estado(t(17, 0), 120).ventana.nombre, "14:00");
        // A las 09:00 la última cerrada fue la de las 02:00
        assert_eq!(estado(t(9, 0), 120).ventana.nombre, "02:00");
    }

    #[test]
    fn con_tolerancia_por_defecto_ninguna_ventana_cruza_medianoche() {
        // Es lo que permite que el día natural baste para agrupar la ronda: si una
        // ventana cruzara las 00:00, la misma ronda caería en dos días distintos y el
        // índice único dejaría pasar un segundo registro del mismo equipo.
        let tol = 119;
        let e_antes = estado(t(23, 59), tol);
        assert!(
            !e_antes.activa,
            "23:59 no debe pertenecer a ninguna ventana"
        );
        let e_despues = estado(t(0, 1), tol);
        assert!(e_despues.activa && e_despues.ventana.nombre == "02:00");
    }

    #[test]
    fn una_tolerancia_excesiva_si_cruza_medianoche_y_debe_rechazarse_en_config() {
        // Documenta el límite: con 180 min la ventana de las 02:00 empieza a las 23:00
        // del día anterior, y ahí el día natural ya no agrupa bien la ronda.
        let e = estado(t(23, 30), 180);
        assert!(
            e.activa,
            "con tolerancia 180 las 23:30 caen dentro de la ronda 02:00"
        );
        assert_eq!(e.ventana.nombre, "02:00");
    }

    #[test]
    fn sin_restriccion_se_mantiene_el_comportamiento_anterior() {
        let diurna = estado_ventana_en("14:00", "02:00", 120, false, t(9, 0)).unwrap();
        assert!(diurna.activa && diurna.ventana.nombre == "14:00");
        let nocturna = estado_ventana_en("14:00", "02:00", 120, false, t(23, 0)).unwrap();
        assert!(nocturna.activa && nocturna.ventana.nombre == "02:00");
    }
}
