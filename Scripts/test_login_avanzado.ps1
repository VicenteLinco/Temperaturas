<#
.SYNOPSIS
    Script de prueba avanzado para Login y Funcionalidad
#>

$BaseUrl = "http://localhost:3000"
$AdminUser = "admin"
$AdminPass = "admin123"

# Configuración de colores
$ColorTitle = "Cyan"
$ColorSuccess = "Green"
$ColorError = "Red"
$ColorInfo = "Yellow"

Clear-Host
Write-Host "`n════════════════════════════════════════════════════" -ForegroundColor $ColorTitle
Write-Host "   TEST AVANZADO: LOGIN Y FUNCIONALIDAD" -ForegroundColor $ColorTitle
Write-Host "════════════════════════════════════════════════════`n"

# 1. Verificar Estado del Servidor
Write-Host "[1/4] Verificando servidor (Puerto 3000)..." -NoNewline
try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $connect = $tcp.BeginConnect("localhost", 3000, $null, $null)
    $wait = $connect.AsyncWaitHandle.WaitOne(2000, $false)
    if ($wait) {
        $tcp.EndConnect($connect)
        $tcp.Close()
        Write-Host " [ONLINE]" -ForegroundColor $ColorSuccess
    } else {
        $tcp.Close()
        Write-Host " [OFFLINE]" -ForegroundColor $ColorError
        Write-Host "`nERROR: El servidor no responde. Ejecuta 'iniciar_servidor.bat' primero." -ForegroundColor $ColorInfo
        Pause
        exit
    }
} catch {
    Write-Host " [ERROR]" -ForegroundColor $ColorError
    Write-Host $_.Exception.Message -ForegroundColor Gray
    exit
}

# 2. Prueba de Login (Admin)
Write-Host "[2/4] Probando Autenticación (Admin)..." -NoNewline
$loginUrl = "$BaseUrl/api/login"
$body = @{ username = $AdminUser; password = $AdminPass } | ConvertTo-Json
$Token = $null

try {
    $response = Invoke-RestMethod -Uri $loginUrl -Method Post -Body $body -ContentType "application/json" -ErrorAction Stop
    Write-Host " [OK]" -ForegroundColor $ColorSuccess
    
    # Analizar respuesta
    if ($response.token) {
        $Token = $response.token
        Write-Host "      Token recibido: " -NoNewline
        Write-Host "$($Token.Substring(0, 15))..." -ForegroundColor Gray
        
        if ($response.role) {
            Write-Host "      Rol detectado: " -NoNewline
            Write-Host $response.role -ForegroundColor $ColorInfo
        }
    } else {
        Write-Host "      ADVERTENCIA: No se recibió token en la respuesta." -ForegroundColor $ColorInfo
        Write-Host "      Respuesta: $($response | ConvertTo-Json -Depth 1)" -ForegroundColor Gray
    }
} catch {
    Write-Host " [FALLO]" -ForegroundColor $ColorError
    Write-Host "      Error: $($_.Exception.Message)" -ForegroundColor Gray
    if ($_.Exception.Response) {
        $reader = New-Object System.IO.StreamReader $_.Exception.Response.GetResponseStream()
        Write-Host "      Detalle: $($reader.ReadToEnd())" -ForegroundColor Gray
    }
}

# 3. Prueba de Acceso Protegido (si hay token)
if ($Token) {
    Write-Host "[3/4] Probando acceso a datos protegidos..." -NoNewline
    $headers = @{ Authorization = "Bearer $Token" }
    
    # Intentamos listar termómetros (endpoint probable)
    $testUrl = "$BaseUrl/api/termometros" 
    
    try {
        $data = Invoke-RestMethod -Uri $testUrl -Method Get -Headers $headers -ErrorAction Stop
        Write-Host " [OK]" -ForegroundColor $ColorSuccess
        if ($data -is [array]) {
            Write-Host "      Registros encontrados: $($data.Count)" -ForegroundColor Gray
            Write-Host "      Campos detectados: " -NoNewline
            if ($data.Count -gt 0) { Write-Host ($data[0].PSObject.Properties.Name -join ", ") -ForegroundColor Yellow }
        } else {
            Write-Host "      Respuesta recibida (Estructura válida)" -ForegroundColor Gray
        }
    } catch {
        Write-Host " [INFO]" -ForegroundColor $ColorInfo
        Write-Host "      No se pudo acceder a '$testUrl' (Quizás la ruta es diferente)" -ForegroundColor Gray
        Write-Host "      Status: $($_.Exception.Response.StatusCode)" -ForegroundColor Gray
    }
} else {
    Write-Host "[3/4] Saltando prueba de datos (Sin token)" -ForegroundColor Gray
}

# 4. Prueba de Seguridad (Login Incorrecto)
Write-Host "[4/4] Verificando rechazo de credenciales falsas..." -NoNewline
$bodyBad = @{ username = "admin"; password = "password_incorrecto_123" } | ConvertTo-Json

try {
    Invoke-RestMethod -Uri $loginUrl -Method Post -Body $bodyBad -ContentType "application/json" -ErrorAction Stop
    Write-Host " [FALLO DE SEGURIDAD]" -ForegroundColor $ColorError
    Write-Host "      El servidor permitió el acceso con contraseña incorrecta." -ForegroundColor $ColorError
} catch {
    $status = $_.Exception.Response.StatusCode
    if ($status -eq [System.Net.HttpStatusCode]::Unauthorized -or $status -eq [System.Net.HttpStatusCode]::Forbidden) {
        Write-Host " [OK] (Acceso denegado correctamente)" -ForegroundColor $ColorSuccess
    } else {
        Write-Host " [?]" -ForegroundColor $ColorInfo
        Write-Host "      Status inesperado: $status" -ForegroundColor Gray
    }
}

Write-Host "`n════════════════════════════════════════════════════" -ForegroundColor $ColorTitle
Write-Host "   PRUEBAS FINALIZADAS" -ForegroundColor $ColorTitle
Write-Host "════════════════════════════════════════════════════`n"
Pause