use serde_json;

#[derive(serde::Serialize, serde::Deserialize)]
struct CrearRegistroRequest {
    pub termometro_id: i64,
    pub temp_actual: Option<f64>,
    pub temp_maxima: f64,
    pub temp_minima: f64,
    pub humedad: Option<f64>,
    pub observaciones: Option<String>,
}

fn main() {
    // Test 1: Con temperatura actual
    let req1 = CrearRegistroRequest {
        termometro_id: 1,
        temp_actual: Some(22.5),
        temp_maxima: 25.0,
        temp_minima: 18.0,
        humedad: Some(65.0),
        observaciones: Some("Test".to_string()),
    };
    
    let json1 = serde_json::to_string_pretty(&req1).unwrap();
    println!("✓ Test 1 - Con temperatura actual:");
    println!("{}\n", json1);
    
    // Test 2: Sin temperatura actual
    let req2 = CrearRegistroRequest {
        termometro_id: 2,
        temp_actual: None,
        temp_maxima: 30.0,
        temp_minima: 20.0,
        humedad: None,
        observaciones: None,
    };
    
    let json2 = serde_json::to_string_pretty(&req2).unwrap();
    println!("✓ Test 2 - Sin temperatura actual:");
    println!("{}\n", json2);
    
    // Test 3: Deserializar JSON
    let json_input = r#"{"termometro_id":3,"temp_actual":23.5,"temp_maxima":26.0,"temp_minima":19.0,"humedad":70.0,"observaciones":"Prueba"}"#;
    let req3: CrearRegistroRequest = serde_json::from_str(json_input).unwrap();
    println!("✓ Test 3 - Deserialización exitosa:");
    println!("  termometro_id: {}", req3.termometro_id);
    println!("  temp_actual: {:?}", req3.temp_actual);
    println!("  temp_maxima: {}", req3.temp_maxima);
    println!("  temp_minima: {}\n", req3.temp_minima);
    
    println!("✅ Todos los tests pasaron correctamente!");
}
