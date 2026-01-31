@echo off
chcp 65001 >nul 2>&1

REM ════════════════════════════════════════════════════════════════
REM   SISTEMA DE GESTIÓN DE TEMPERATURAS - Modo Servicio
REM   Ejecutándose en segundo plano con icono en bandeja del sistema
REM ════════════════════════════════════════════════════════════════

REM Crear archivo de log
set LOGFILE=%~dp0servidor.log
echo [%date% %time%] Iniciando Sistema de Temperaturas... > "%LOGFILE%"

REM Verificar si el servidor ya está corriendo
netstat -ano | findstr :3000 >nul 2>&1
if %errorlevel% equ 0 (
    echo [%date% %time%] ADVERTENCIA: El servidor ya está corriendo en el puerto 3000 >> "%LOGFILE%"
    echo [%date% %time%] Deteniendo proceso anterior... >> "%LOGFILE%"
    for /f "tokens=5" %%a in ('netstat -ano ^| findstr :3000 ^| findstr LISTENING') do taskkill /PID %%a /F >nul 2>&1
    timeout /t 2 /nobreak >nul
)

REM ═══ NGROK (OPCIONAL) ═══
REM Verificar si existe el token en .env o token.txt
set NGROK_TOKEN=
if exist "%~dp0..\.env" (
    for /f "tokens=2 delims==" %%a in ('findstr /B "NGROK_AUTHTOKEN=" "%~dp0..\.env"') do set NGROK_TOKEN=%%a
)
if "%NGROK_TOKEN%"=="" (
    if exist "%~dp0token.txt" (
        set /p NGROK_TOKEN=<"%~dp0token.txt"
    )
)

set USE_NGROK=0
if not "%NGROK_TOKEN%"=="" (
    set USE_NGROK=1
    echo [%date% %time%] Token de ngrok encontrado, habilitando tunel publico... >> "%LOGFILE%"
) else (
    echo [%date% %time%] No se encontro token de ngrok, modo solo local >> "%LOGFILE%"
)

if %USE_NGROK%==1 (
    REM Verificar si ngrok ya está corriendo
    tasklist | findstr ngrok.exe >nul 2>&1
    if %errorlevel% equ 0 (
        echo [%date% %time%] ADVERTENCIA: ngrok ya está corriendo >> "%LOGFILE%"
        echo [%date% %time%] Deteniendo proceso anterior... >> "%LOGFILE%"
        taskkill /IM ngrok.exe /F >nul 2>&1
        timeout /t 2 /nobreak >nul
    )

    REM Configurar ngrok con token
    echo [%date% %time%] Configurando ngrok... >> "%LOGFILE%"
    if exist "%~dp0ngrok.exe" (
        "%~dp0ngrok.exe" config add-authtoken %NGROK_TOKEN% >nul 2>&1
    )
)

REM Iniciar servidor Rust
echo [%date% %time%] Iniciando servidor Rust... >> "%LOGFILE%"
cd /d "%~dp0.."
start /B cargo run >> "%LOGFILE%" 2>&1

REM Esperar que el servidor esté listo
echo [%date% %time%] Esperando que el servidor esté listo... >> "%LOGFILE%"
timeout /t 7 /nobreak >nul

REM Iniciar ngrok si está habilitado
if %USE_NGROK%==1 (
    echo [%date% %time%] Iniciando ngrok... >> "%LOGFILE%"
    start "" /B "%~dp0ngrok.exe" http 3000
    timeout /t 5 /nobreak >nul

    REM Mostrar notificación con URL pública
    echo [%date% %time%] Obteniendo URL publica... >> "%LOGFILE%"
    powershell -WindowStyle Hidden -Command "$response = try { Invoke-RestMethod -Uri 'http://localhost:4040/api/tunnels' -ErrorAction Stop } catch { $null }; if($response) { $url = $response.tunnels[0].public_url; Add-Type -AssemblyName System.Windows.Forms; $global:balloon = New-Object System.Windows.Forms.NotifyIcon; $path = (Get-Process -id $pid).Path; $balloon.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($path); $balloon.BalloonTipIcon = [System.Windows.Forms.ToolTipIcon]::Info; $balloon.BalloonTipText = \"URL Publica: $url`n`nLocal: http://localhost:3000\"; $balloon.BalloonTipTitle = 'Sistema de Temperaturas Iniciado'; $balloon.Visible = $true; $balloon.ShowBalloonTip(10000); Start-Sleep -Seconds 1; } else { Add-Type -AssemblyName System.Windows.Forms; $global:balloon = New-Object System.Windows.Forms.NotifyIcon; $path = (Get-Process -id $pid).Path; $balloon.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($path); $balloon.BalloonTipIcon = [System.Windows.Forms.ToolTipIcon]::Info; $balloon.BalloonTipText = 'Servidor corriendo en http://localhost:3000'; $balloon.BalloonTipTitle = 'Sistema de Temperaturas'; $balloon.Visible = $true; $balloon.ShowBalloonTip(8000); Start-Sleep -Seconds 1; }"
) else (
    REM Mostrar notificación sin ngrok (solo local)
    echo [%date% %time%] Mostrando notificación (modo local)... >> "%LOGFILE%"
    powershell -WindowStyle Hidden -Command "Add-Type -AssemblyName System.Windows.Forms; $global:balloon = New-Object System.Windows.Forms.NotifyIcon; $path = (Get-Process -id $pid).Path; $balloon.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($path); $balloon.BalloonTipIcon = [System.Windows.Forms.ToolTipIcon]::Info; $balloon.BalloonTipText = 'Servidor corriendo en http://localhost:3000`n`nModo: Solo Local'; $balloon.BalloonTipTitle = 'Sistema de Temperaturas Iniciado'; $balloon.Visible = $true; $balloon.ShowBalloonTip(8000); Start-Sleep -Seconds 1;"
)

echo [%date% %time%] Servicios iniciados correctamente >> "%LOGFILE%"

REM Crear icono en bandeja del sistema con menú contextual
powershell -WindowStyle Hidden -Command "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $appContext = New-Object System.Windows.Forms.ApplicationContext; $notifyIcon = New-Object System.Windows.Forms.NotifyIcon; $path = (Get-Process -id $pid).Path; $notifyIcon.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon($path); $notifyIcon.Text = 'Sistema de Temperaturas - Activo'; $notifyIcon.Visible = $true; $contextMenu = New-Object System.Windows.Forms.ContextMenuStrip; $menuItemAbrir = New-Object System.Windows.Forms.ToolStripMenuItem; $menuItemAbrir.Text = 'Abrir en navegador'; $menuItemAbrir.Add_Click({ try { $response = Invoke-RestMethod -Uri 'http://localhost:4040/api/tunnels' -ErrorAction SilentlyContinue; if($response -and $response.tunnels.Count -gt 0) { Start-Process $response.tunnels[0].public_url } else { Start-Process 'http://localhost:3000' } } catch { Start-Process 'http://localhost:3000' } }); $menuItemLocal = New-Object System.Windows.Forms.ToolStripMenuItem; $menuItemLocal.Text = 'Abrir local (localhost:3000)'; $menuItemLocal.Add_Click({ Start-Process 'http://localhost:3000' }); $menuItemStatus = New-Object System.Windows.Forms.ToolStripMenuItem; $menuItemStatus.Text = 'Ver estado'; $menuItemStatus.Add_Click({ try { $response = Invoke-RestMethod -Uri 'http://localhost:4040/api/tunnels' -ErrorAction SilentlyContinue; if($response) { $url = $response.tunnels[0].public_url; [System.Windows.Forms.MessageBox]::Show(\"Estado: Activo`n`nURL Pública: $url`n`nURL Local: http://localhost:3000\", 'Sistema de Temperaturas', [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information) } else { [System.Windows.Forms.MessageBox]::Show(\"Estado: Activo (solo local)`n`nURL Local: http://localhost:3000`n`nNota: ngrok no está disponible\", 'Sistema de Temperaturas', [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information) } } catch { [System.Windows.Forms.MessageBox]::Show(\"Estado: Activo`n`nURL Local: http://localhost:3000\", 'Sistema de Temperaturas', [System.Windows.Forms.MessageBoxButtons]::OK, [System.Windows.Forms.MessageBoxIcon]::Information) } }); $menuItemLog = New-Object System.Windows.Forms.ToolStripMenuItem; $menuItemLog.Text = 'Ver log'; $menuItemLog.Add_Click({ Start-Process notepad.exe -ArgumentList '%LOGFILE%' }); $menuItemSeparator = New-Object System.Windows.Forms.ToolStripSeparator; $menuItemSalir = New-Object System.Windows.Forms.ToolStripMenuItem; $menuItemSalir.Text = 'Detener servidor'; $menuItemSalir.Add_Click({ $result = [System.Windows.Forms.MessageBox]::Show('¿Está seguro que desea detener el servidor?', 'Confirmar', [System.Windows.Forms.MessageBoxButtons]::YesNo, [System.Windows.Forms.MessageBoxIcon]::Question); if($result -eq [System.Windows.Forms.DialogResult]::Yes) { Get-Process | Where-Object {$_.ProcessName -like '*cargo*' -or $_.ProcessName -like '*rust*' -or $_.ProcessName -eq 'ngrok'} | Stop-Process -Force -ErrorAction SilentlyContinue; $notifyIcon.Visible = $false; [System.Windows.Forms.Application]::Exit() } }); $contextMenu.Items.Add($menuItemAbrir); $contextMenu.Items.Add($menuItemLocal); $contextMenu.Items.Add($menuItemStatus); $contextMenu.Items.Add($menuItemLog); $contextMenu.Items.Add($menuItemSeparator); $contextMenu.Items.Add($menuItemSalir); $notifyIcon.ContextMenuStrip = $contextMenu; $notifyIcon.Add_DoubleClick({ try { $response = Invoke-RestMethod -Uri 'http://localhost:4040/api/tunnels' -ErrorAction SilentlyContinue; if($response -and $response.tunnels.Count -gt 0) { Start-Process $response.tunnels[0].public_url } else { Start-Process 'http://localhost:3000' } } catch { Start-Process 'http://localhost:3000' } }); [System.Windows.Forms.Application]::Run($appContext)"

REM Este punto se alcanza solo si se cierra el icono de bandeja
echo [%date% %time%] Cerrando servicios... >> "%LOGFILE%"
taskkill /IM ngrok.exe /F >nul 2>&1
for /f "tokens=5" %%a in ('netstat -ano ^| findstr :3000 ^| findstr LISTENING') do taskkill /PID %%a /F >nul 2>&1

echo [%date% %time%] Servicios detenidos >> "%LOGFILE%"
exit
