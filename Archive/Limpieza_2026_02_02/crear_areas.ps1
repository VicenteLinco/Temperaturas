# Este script automatiza la creación de áreas técnicas en el sistema.
# Lee una lista de nombres de áreas y envía una solicitud POST a la API para cada una.
#
# REQUISITO:
# 1. Iniciar sesión en la aplicación web como administrador para tener una sesión activa.
# 2. El script utilizará las cookies de sesión guardadas en `cookies.txt`.

# --- Configuración ---
$apiUrl = "http://localhost:3000/api/admin/areas"
$cookieFile = "cookies.txt"

# --- Áreas a Crear ---
$areas = @(
    "BIOQUIMICA",
    "UMT",
    "SEROLOGIA",
    "RECEPCION",
    "PASILLO",
    "LBM",
    "MICRO",
    "ORINAS",
    "TBC",
    "SALA DE LAVADO",
    "BODEGAS",
    "DONANTES"
)

# --- Lógica del Script ---

# Verificar que el archivo de cookies existe
if (-not (Test-Path $cookieFile)) {
    Write-Error "Error: El archivo de cookies '$cookieFile' no fue encontrado."
    Write-Error "Por favor, inicie sesión en la aplicación primero para generar el archivo."
    exit
}

# Crear una sesión web
$session = New-Object Microsoft.PowerShell.Commands.WebRequestSession

# Leer y procesar el archivo de cookies (formato Netscape)
try {
    Get-Content $cookieFile | ForEach-Object {
        if ($_ -notmatch '^#' -and $_ -match "`t") {
            $parts = $_.Split("`t")
            $cookie = New-Object System.Net.Cookie
            
            $domain = $parts[0]
            if ($domain.StartsWith("#HttpOnly_")) {
                $domain = $domain.Substring(10)
                $cookie.HttpOnly = $true
            } else {
                $cookie.HttpOnly = $false
            }

            $cookie.Domain = $domain
            $cookie.Path = $parts[2]
            $cookie.Secure = ($parts[3] -eq 'TRUE')
            $cookie.Name = $parts[5]
            $cookie.Value = $parts[6]
            
            $session.Cookies.Add($cookie)
        }
    }
} catch {
    Write-Error "Error al procesar el archivo de cookies: $($_.Exception.Message)"
    exit
}


Write-Host "Iniciando la creación de ${areas.Count} áreas..."

foreach ($areaNombre in $areas) {
    Write-Host "  - Creando área: $areaNombre"

    $body = @{
        nombre = $areaNombre
        descripcion = "Área técnica importada automáticamente."
    } | ConvertTo-Json

    try {
        $response = Invoke-RestMethod -Uri $apiUrl -Method Post -Body $body -ContentType "application/json" -WebSession $session

        if ($response) {
            Write-Host "    ✅ Área '$($response.nombre)' creada con éxito (ID: $($response.id))."
        } else {
            Write-Host "    ✅ Solicitud para crear '$areaNombre' enviada con éxito (Respuesta sin cuerpo)."
        }
    } catch {
        # Capturar y mostrar errores de la solicitud
        $statusCode = 0
        if ($_.Exception.Response) {
            $statusCode = $_.Exception.Response.StatusCode.value__
        }

        $errorMessage = ""
        try {
            if ($_.Exception.Response) {
                $errorResponse = $_.Exception.Response.Content | ConvertFrom-Json
                $errorMessage = $errorResponse.message
            } else {
                $errorMessage = $_.Exception.Message
            }
        } catch {
            $errorMessage = $_.Exception.Message
        }

        if ($statusCode -eq 409) { # Conflicto
             Write-Warning "    ⚠️  El área '$areaNombre' ya existe en el sistema."
        } else {
             Write-Error "    ❌ Error creando el área '$areaNombre' (Código: $statusCode)."
             Write-Error "       Mensaje: $errorMessage"
        }
    }
}

Write-Host "Proceso de creación de áreas finalizado."
