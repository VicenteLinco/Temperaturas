# Script de Requisitos para el Sistema de Temperaturas
Write-Host "===============================================" -ForegroundColor Cyan
Write-Host "   VERIFICADOR DE REQUISITOS DEL SISTEMA" -ForegroundColor Cyan
Write-Host "===============================================" -ForegroundColor Cyan

# 1. Verificar Rust
if (Get-Command "cargo" -ErrorAction SilentlyContinue) {
    Write-Host "[OK] Rust y Cargo están instalados." -ForegroundColor Green
} else {
    Write-Host "[!] Rust no encontrado. Instalando rustup..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile "$env:TEMP\rustup-init.exe"
    Start-Process "$env:TEMP\rustup-init.exe" -Wait
}

# 2. Verificar Herramientas de Compilación C++ (Linker)
$vsPath = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($vsPath) {
    Write-Host "[OK] Herramientas de compilación C++ (MSVC) detectadas." -ForegroundColor Green
} else {
    Write-Host "[ERROR] Faltan las Build Tools de Visual Studio." -ForegroundColor Red
    Write-Host "Descargue e instale desde: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor White
    Write-Host "Seleccione la carga de trabajo: 'Desarrollo para el escritorio con C++'" -ForegroundColor Yellow
}

# 3. Verificar Archivo .env
if (Test-Path ".env") {
    Write-Host "[OK] Archivo de configuración .env detectado." -ForegroundColor Green
} else {
    Write-Host "[!] No se encontró .env. Creando uno desde .env.example..." -ForegroundColor Yellow
    Copy-Item ".env.example" ".env"
    Write-Host "POR FAVOR: Edita el archivo .env con tu token de ngrok." -ForegroundColor Cyan
}

# 4. Compilación Inicial
Write-Host "`n¿Desea realizar la compilación inicial ahora? (S/N)" -NoNewline
$resp = Read-Host
if ($resp -eq "S" -or $resp -eq "s") {
    Write-Host "Compilando... Esto puede tardar varios minutos la primera vez." -ForegroundColor Cyan
    cargo build
    if ($LASTEXITCODE -eq 0) {
        Write-Host "[OK] Sistema compilado correctamente." -ForegroundColor Green
    } else {
        Write-Host "[ERROR] Falló la compilación. Verifique los errores arriba." -ForegroundColor Red
    }
}

Write-Host "`nProceso finalizado. Presione cualquier tecla para salir."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
