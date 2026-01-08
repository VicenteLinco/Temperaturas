use chrono::{NaiveTime, NaiveDate, Local};
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

/// Determina la ventana horaria actual basándose en la hora del sistema
/// Con el nuevo sistema flexible, siempre devuelve una ventana dependiendo del turno
pub fn determinar_ventana_actual(
    hora_1: &str,  // "14:00" turno día
    hora_2: &str,  // "02:00" turno noche
    _tolerancia_minutos: i32, // No se usa en el nuevo sistema flexible
) -> Result<Option<VentanaHoraria>> {
    let ahora = Local::now().time();

    // Parsear horas configuradas
    let hora_central_1 = NaiveTime::parse_from_str(hora_1, "%H:%M")?;
    let hora_central_2 = NaiveTime::parse_from_str(hora_2, "%H:%M")?;

    // Determinar en qué turno estamos
    // Turno día: 8:00 - 20:00 → ventana 14:00
    // Turno noche: 20:00 - 8:00 → ventana 02:00

    let hora_inicio_dia = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
    let hora_fin_dia = NaiveTime::from_hms_opt(20, 0, 0).unwrap();

    if ahora >= hora_inicio_dia && ahora < hora_fin_dia {
        // Turno día
        Ok(Some(VentanaHoraria {
            nombre: hora_1.to_string(),
            hora_central: hora_central_1,
            hora_inicio: hora_inicio_dia,
            hora_fin: hora_fin_dia,
            es_turno_noche: false,
        }))
    } else {
        // Turno noche
        Ok(Some(VentanaHoraria {
            nombre: hora_2.to_string(),
            hora_central: hora_central_2,
            hora_inicio: hora_fin_dia, // 20:00
            hora_fin: hora_inicio_dia, // 8:00 (del día siguiente)
            es_turno_noche: true,
        }))
    }
}

/// Calcula el día asignado para un registro basándose en el turno
/// - Turno día (8am-20pm): El día asignado es el día actual
/// - Turno noche (20pm-8am): El día asignado es el día siguiente
#[allow(dead_code)]
pub fn calcular_dia_asignado(ventana: &VentanaHoraria, fecha_registro: &chrono::NaiveDateTime) -> NaiveDate {
    let fecha_actual = fecha_registro.date();

    if ventana.es_turno_noche {
        // Para el turno noche, si estamos después de las 20:00, el día asignado es mañana
        // Si estamos antes de las 8:00 (madrugada), el día asignado es hoy
        let hora_registro = fecha_registro.time();
        let hora_corte = NaiveTime::from_hms_opt(20, 0, 0).unwrap();

        if hora_registro >= hora_corte {
            // Estamos entre 20:00-23:59, día asignado es mañana
            fecha_actual + chrono::Duration::days(1)
        } else {
            // Estamos entre 00:00-07:59, día asignado es hoy
            fecha_actual
        }
    } else {
        // Turno día: día asignado es el día actual
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
    temp: f64,
    tipo: &TipoTermometro,
    campo: &str, // "máxima" o "mínima"
) -> ValidacionResultado {
    // Verificar límites físicos (rechaza)
    if temp < tipo.temp_min_fisica || temp > tipo.temp_max_fisica {
        return ValidacionResultado::Rechazo(format!(
            "Temperatura {} ({:.1}°C) fuera de rango físico ({:.1}°C a {:.1}°C)",
            campo, temp, tipo.temp_min_fisica, tipo.temp_max_fisica
        ));
    }

    // Verificar límites operativos (advertencia)
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
    humedad: f64,
    tipo: &TipoTermometro,
) -> Result<ValidacionResultado> {
    if !tipo.tiene_humedad {
        return Err(anyhow!("Este tipo de termómetro no mide humedad"));
    }

    let hum_min_fisica = tipo.hum_min_fisica.ok_or(anyhow!("Rango físico de humedad no configurado"))?;
    let hum_max_fisica = tipo.hum_max_fisica.ok_or(anyhow!("Rango físico de humedad no configurado"))?;
    let hum_min_operativa = tipo.hum_min_operativa.ok_or(anyhow!("Rango operativo de humedad no configurado"))?;
    let hum_max_operativa = tipo.hum_max_operativa.ok_or(anyhow!("Rango operativo de humedad no configurado"))?;

    // Verificar límites físicos (rechaza)
    if humedad < hum_min_fisica || humedad > hum_max_fisica {
        return Ok(ValidacionResultado::Rechazo(format!(
            "Humedad ({:.1}%) fuera de rango físico ({:.1}% a {:.1}%)",
            humedad, hum_min_fisica, hum_max_fisica
        )));
    }

    // Verificar límites operativos (advertencia)
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
    temp_maxima: f64,
    temp_minima: f64,
    humedad: Option<f64>,
    tipo: &TipoTermometro,
) -> Result<(bool, Vec<String>)> {
    let mut advertencias = Vec::new();
    let mut fuera_rango_operativo = false;

    // ✅ NUEVA VALIDACIÓN: Verificar coherencia entre máxima y mínima
    if temp_maxima < temp_minima {
        return Err(anyhow!(
            "Temperatura máxima ({:.1}°C) no puede ser menor que temperatura mínima ({:.1}°C)",
            temp_maxima, temp_minima
        ));
    }

    // Validar temperatura máxima
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

    // Validar temperatura mínima
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

    // Verificar que temp_maxima >= temp_minima
    if temp_maxima < temp_minima {
        return Err(anyhow!("La temperatura máxima no puede ser menor que la mínima"));
    }

    // Validar humedad si está presente
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinar_ventana_turno_dia() {
        // Este test necesitaría mockear la hora actual para ser determinístico
        // Por ahora lo dejamos como placeholder
    }

    #[test]
    fn test_calcular_dia_asignado() {
        let ventana_noche = VentanaHoraria {
            nombre: "02:00".to_string(),
            hora_central: NaiveTime::from_hms_opt(2, 0, 0).unwrap(),
            hora_inicio: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            hora_fin: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            es_turno_noche: true,
        };

        // Lunes 22:00 → Martes
        let fecha_lunes_noche = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
            .and_hms_opt(22, 0, 0).unwrap();
        let dia_asignado = calcular_dia_asignado(&ventana_noche, &fecha_lunes_noche);
        assert_eq!(dia_asignado, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());

        // Martes 03:00 → Martes
        let fecha_martes_madrugada = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()
            .and_hms_opt(3, 0, 0).unwrap();
        let dia_asignado = calcular_dia_asignado(&ventana_noche, &fecha_martes_madrugada);
        assert_eq!(dia_asignado, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
    }
}
