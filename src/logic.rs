use chrono::{NaiveTime, NaiveDate, Local, Timelike};
use anyhow::{Result, anyhow};
use crate::models::TipoTermometro;

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
    hora_1: &str,  // "14:00" turno día
    hora_2: &str,  // "02:00" turno noche
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

/// Calcula el día asignado para un registro basándose en el turno
#[allow(dead_code)]
pub fn calcular_dia_asignado(ventana: &VentanaHoraria, fecha_registro: &chrono::NaiveDateTime) -> NaiveDate {
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
pub fn validar_humedad(
    humedad: f32,
    tipo: &TipoTermometro,
) -> Result<ValidacionResultado> {
    if !tipo.tiene_humedad {
        return Err(anyhow!("Este tipo de termómetro no mide humedad"));
    }

    let hum_min_fisica = tipo.hum_min_fisica.ok_or(anyhow!("Rango físico de humedad no configurado"))?;
    let hum_max_fisica = tipo.hum_max_fisica.ok_or(anyhow!("Rango físico de humedad no configurado"))?;
    let hum_min_operativa = tipo.hum_min_operativa.ok_or(anyhow!("Rango operativo de humedad no configurado"))?;
    let hum_max_operativa = tipo.hum_max_operativa.ok_or(anyhow!("Rango operativo de humedad no configurado"))?;

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

/// Valida un registro completo
pub fn validar_registro(
    temp_maxima: f32,
    temp_minima: f32,
    humedad: Option<f32>,
    tipo: &TipoTermometro,
) -> Result<(bool, Vec<String>)> {
    let mut advertencias = Vec::new();
    let mut fuera_rango_operativo = false;

    if temp_maxima < temp_minima {
        return Err(anyhow!(
            "Temperatura máxima ({:.1}°C) no puede ser menor que temperatura mínima ({:.1}°C)",
            temp_maxima, temp_minima
        ));
    }

    match validar_temperatura(temp_maxima, tipo, "máxima") {
        ValidacionResultado::Ok => {},
        ValidacionResultado::Advertencia(msg) => {
            advertencias.push(msg);
            fuera_rango_operativo = true;
        },
        ValidacionResultado::Rechazo(msg) => {
            return Err(anyhow!(msg));
        },
    }

    match validar_temperatura(temp_minima, tipo, "mínima") {
        ValidacionResultado::Ok => {},
        ValidacionResultado::Advertencia(msg) => {
            advertencias.push(msg);
            fuera_rango_operativo = true;
        },
        ValidacionResultado::Rechazo(msg) => {
            return Err(anyhow!(msg));
        },
    }

    if let Some(h) = humedad {
        match validar_humedad(h, tipo)? {
            ValidacionResultado::Ok => {},
            ValidacionResultado::Advertencia(msg) => {
                advertencias.push(msg);
                fuera_rango_operativo = true;
            },
            ValidacionResultado::Rechazo(msg) => {
                return Err(anyhow!(msg));
            },
        }
    } else if tipo.tiene_humedad {
        return Err(anyhow!("Este tipo de termómetro requiere medición de humedad"));
    }

    Ok((fuera_rango_operativo, advertencias))
}

/// Ordena las lecturas para que la máxima sea la mayor y la mínima la menor.
/// Los registradores a veces confunden máx/mín cuando hay números negativos
/// (p. ej. registran -12 en mínima y -18 en máxima). Al reordenar aquí en el
/// backend, la data siempre queda coherente sin importar el orden de ingreso.
pub fn normalizar_lecturas(
    temp_actual: Option<f32>,
    lectura_a: f32,
    lectura_b: f32,
) -> (Option<f32>, f32, f32) {
    let mut valores = Vec::with_capacity(3);
    if let Some(a) = temp_actual {
        valores.push(a);
    }
    valores.push(lectura_a);
    valores.push(lectura_b);

    let temp_minima = valores.iter().cloned().fold(f32::INFINITY, f32::min);
    let temp_maxima = valores.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

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
        assert!(matches!(validar_temperatura(5.0, &tipo, "máxima"), ValidacionResultado::Ok));
    }

    #[test]
    fn validar_temperatura_advertencia() {
        let tipo = tipo_base();
        assert!(matches!(validar_temperatura(20.0, &tipo, "máxima"), ValidacionResultado::Advertencia(_)));
    }

    #[test]
    fn validar_temperatura_rechazo_fisico() {
        let tipo = tipo_base();
        assert!(matches!(validar_temperatura(-50.0, &tipo, "mínima"), ValidacionResultado::Rechazo(_)));
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
        let res = validar_registro(3.0, 5.0, None, &tipo);
        assert!(res.is_err());
    }

    #[test]
    fn validar_registro_ok() {
        let tipo = tipo_base();
        let (fuera, advertencias) = validar_registro(6.0, 4.0, None, &tipo).unwrap();
        assert!(!fuera);
        assert!(advertencias.is_empty());
    }

    #[test]
    fn validar_registro_humedad_requerida() {
        let tipo = tipo_con_humedad();
        assert!(validar_registro(6.0, 4.0, None, &tipo).is_err());
    }

    #[test]
    fn validar_registro_ok_con_humedad() {
        let tipo = tipo_con_humedad();
        let (fuera, advertencias) = validar_registro(6.0, 4.0, Some(50.0), &tipo).unwrap();
        assert!(!fuera);
        assert!(advertencias.is_empty());
    }

    #[test]
    fn validar_registro_marca_fuera_rango() {
        let tipo = tipo_base();
        let (fuera, advertencias) = validar_registro(20.0, 4.0, None, &tipo).unwrap();
        assert!(fuera);
        assert!(!advertencias.is_empty());
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
    fn normalizar_lecturas_actual_puede_ser_extrema() {
        let (_, maxima, minima) = normalizar_lecturas(Some(-25.0), -12.0, -18.0);
        assert_eq!(maxima, -12.0);
        assert_eq!(minima, -25.0);
    }

    #[test]
    fn normalizar_lecturas_sin_actual() {
        let (actual, maxima, minima) = normalizar_lecturas(None, -12.0, -18.0);
        assert_eq!(actual, None);
        assert_eq!(maxima, -12.0);
        assert_eq!(minima, -18.0);
    }
}
