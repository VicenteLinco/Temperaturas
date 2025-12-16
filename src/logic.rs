use chrono::{NaiveTime, Local, Timelike};
use anyhow::{Result, anyhow};
use crate::models::TipoTermometro;

/// Representa una ventana horaria de registro
#[derive(Debug, Clone)]
pub struct VentanaHoraria {
    pub nombre: String, // "14:00" o "02:00"
    pub hora_central: NaiveTime,
    pub hora_inicio: NaiveTime,
    pub hora_fin: NaiveTime,
}

/// Determina la ventana horaria actual basándose en la hora del sistema
pub fn determinar_ventana_actual(
    hora_1: &str,
    hora_2: &str,
    tolerancia_minutos: i32,
) -> Result<Option<VentanaHoraria>> {
    let ahora = Local::now().time();

    // Parsear horas configuradas
    let hora_central_1 = NaiveTime::parse_from_str(hora_1, "%H:%M")?;
    let hora_central_2 = NaiveTime::parse_from_str(hora_2, "%H:%M")?;

    // Calcular ventanas
    let ventana_1 = calcular_ventana(hora_1, hora_central_1, tolerancia_minutos)?;
    let ventana_2 = calcular_ventana(hora_2, hora_central_2, tolerancia_minutos)?;

    // Verificar si estamos en alguna ventana
    if esta_en_ventana(&ahora, &ventana_1) {
        return Ok(Some(ventana_1));
    }

    if esta_en_ventana(&ahora, &ventana_2) {
        return Ok(Some(ventana_2));
    }

    Ok(None)
}

/// Calcula los límites de una ventana horaria
fn calcular_ventana(
    nombre: &str,
    hora_central: NaiveTime,
    tolerancia_minutos: i32,
) -> Result<VentanaHoraria> {
    // Calcular minutos totales desde medianoche
    let minutos_centrales = hora_central.hour() as i32 * 60 + hora_central.minute() as i32;

    // Calcular inicio y fin
    let minutos_inicio = minutos_centrales - tolerancia_minutos;
    let minutos_fin = minutos_centrales + tolerancia_minutos;

    // Manejar wrap-around de medianoche
    let hora_inicio = minutos_a_tiempo(if minutos_inicio < 0 {
        1440 + minutos_inicio // 1440 = 24 * 60
    } else {
        minutos_inicio
    });

    let hora_fin = minutos_a_tiempo(if minutos_fin >= 1440 {
        minutos_fin - 1440
    } else {
        minutos_fin
    });

    Ok(VentanaHoraria {
        nombre: nombre.to_string(),
        hora_central,
        hora_inicio,
        hora_fin,
    })
}

/// Convierte minutos desde medianoche a NaiveTime
fn minutos_a_tiempo(minutos: i32) -> NaiveTime {
    let horas = (minutos / 60) % 24;
    let mins = minutos % 60;
    NaiveTime::from_hms_opt(horas as u32, mins as u32, 0).unwrap()
}

/// Verifica si una hora está dentro de una ventana
fn esta_en_ventana(hora: &NaiveTime, ventana: &VentanaHoraria) -> bool {
    if ventana.hora_inicio <= ventana.hora_fin {
        // Ventana no cruza medianoche
        *hora >= ventana.hora_inicio && *hora <= ventana.hora_fin
    } else {
        // Ventana cruza medianoche (ej: 23:00 - 03:00)
        *hora >= ventana.hora_inicio || *hora <= ventana.hora_fin
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
    fn test_calcular_ventana() {
        let ventana = calcular_ventana("14:00", NaiveTime::from_hms_opt(14, 0, 0).unwrap(), 119).unwrap();
        assert_eq!(ventana.hora_inicio, NaiveTime::from_hms_opt(12, 1, 0).unwrap());
        assert_eq!(ventana.hora_fin, NaiveTime::from_hms_opt(15, 59, 0).unwrap());
    }

    #[test]
    fn test_ventana_cruza_medianoche() {
        let ventana = calcular_ventana("02:00", NaiveTime::from_hms_opt(2, 0, 0).unwrap(), 119).unwrap();
        assert_eq!(ventana.hora_inicio, NaiveTime::from_hms_opt(0, 1, 0).unwrap());
        assert_eq!(ventana.hora_fin, NaiveTime::from_hms_opt(3, 59, 0).unwrap());
    }
}
