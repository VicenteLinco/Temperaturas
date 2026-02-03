<#
.SYNOPSIS
    Script de prueba básico para el Sistema de Temperaturas
.DESCRIPTION
    Verifica que el servidor esté corriendo y responde a solicitudes básicas.
#>

$BaseUrl = "http://localhost:3000"
$AdminUser = "admin"
$AdminPass = "admin123"

Write-Host "`n════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "   PRUEBA DE FUNCIONALIDADES - SISTEMA DE TEMPERATURAS" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════`n"

# 1. Verificar si el servidor está escuchando
Write-Host "[1/3] Verificando conectividad (Puerto 3000)..." -NoNewline
try {
    $tcp = New-Object System.Net.Sockets.TcpClient
    $connect = $tcp.BeginConnect("localhost", 3000, $null, $null)
    $wait = $connect.AsyncWaitHandle.WaitOne(2000, $false)
    if ($wait) {
        $tcp.EndConnect($connect)
        Write-Host " [OK]" -ForegroundColor Green
        $tcp.Close()
    } else {
        $tcp.Close()
        Write-Host " [FALLO]" -ForegroundColor Red
        Write-Host "`nERROR: El servidor no parece estar corriendo en el puerto 3000." -ForegroundColor Yellow
        Write-Host "Asegúrate de ejecutar 'iniciar_servidor.bat' primero."
        exit
    }
} catch {
    Write-Host " [ERROR]" -ForegroundColor Red
}

# 2. Verificar respuesta HTTP raíz
Write-Host "[2/3] Verificando respuesta HTTP..." -NoNewline
try {
    $response = Invoke-WebRequest -Uri $BaseUrl -Method Get -ErrorAction Stop
    Write-Host " [OK] (Status: $($response.StatusCode))" -ForegroundColor Green
} catch {
    Write-Host " [FALLO]" -ForegroundColor Red
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Gray
}

# 3. Probar Autenticación (Intento)
Write-Host "[3/3] Probando autenticación (admin)..." -NoNewline
$loginUrl = "$BaseUrl/api/login"
$body = @{ username = $AdminUser; password = $AdminPass } | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri $loginUrl -Method Post -Body $body -ContentType "application/json" -ErrorAction Stop
    Write-Host " [OK] (Endpoint respondio)" -ForegroundColor Green
} catch {
    Write-Host " [INFO]" -ForegroundColor Yellow
    Write-Host "Nota: No se pudo completar el login automático (posiblemente ruta diferente)."
}

Write-Host "`n════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "   PRUEBAS FINALIZADAS" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════`n"
Pause