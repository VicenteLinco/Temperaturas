
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# --- Win32 API para ocultar/mostrar ventanas ---
$win32 = @"
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("kernel32.dll")]
    public static extern IntPtr GetConsoleWindow();
"@

try {
    $win32Funcs = Add-Type -MemberDefinition $win32 -Name "Win32WindowFuncs" -Namespace Win32Functions -PassThru
} catch {
    $win32Funcs = [Win32Functions.Win32WindowFuncs]
}

# Constantes de ShowWindow
$SW_HIDE = 0
$SW_SHOW = 5

# Obtener el handle de la consola ACTUAL (metodo robusto)
$consolePtr = $win32Funcs::GetConsoleWindow()

# Intentar encontrar la ventana de "Servidor Rust"
$rustProcess = Get-Process | Where-Object { $_.MainWindowTitle -eq "Servidor Rust" } | Select-Object -First 1

# --- Función para ocultar todo ---
function Hide-All {
    if ($consolePtr -ne [IntPtr]::Zero) {
        $win32Funcs::ShowWindow($consolePtr, $SW_HIDE) | Out-Null
    }
    if ($rustProcess) {
        $win32Funcs::ShowWindow($rustProcess.MainWindowHandle, $SW_HIDE) | Out-Null
    }
}

# --- Función para restaurar todo ---
function Show-All {
    if ($consolePtr -ne [IntPtr]::Zero) {
        $win32Funcs::ShowWindow($consolePtr, $SW_SHOW) | Out-Null
        $win32Funcs::SetForegroundWindow($consolePtr) | Out-Null
    }
    if ($rustProcess) {
        $win32Funcs::ShowWindow($rustProcess.MainWindowHandle, $SW_SHOW) | Out-Null
    }
}

# --- Configuración del Icono de Bandeja ---
$notifyIcon = New-Object System.Windows.Forms.NotifyIcon
$iconPath = "$PSScriptRoot\icons8-peligro-de-alta-temperatura-windows-11-color-16.png"

# Setup Icon
if (Test-Path $iconPath) {
    try {
        $bitmap = [System.Drawing.Bitmap]::FromFile($iconPath)
        $iconHandle = $bitmap.GetHicon()
        $notifyIcon.Icon = [System.Drawing.Icon]::FromHandle($iconHandle)
    } catch {
        $notifyIcon.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon((Get-Process -Id $PID).Path)
    }
} else {
    try {
        $notifyIcon.Icon = [System.Drawing.Icon]::ExtractAssociatedIcon((Get-Process -Id $PID).Path)
    } catch {
        $notifyIcon.Icon = [System.Drawing.SystemIcons]::Application
    }
}

$notifyIcon.Text = "Sistema de Temperaturas (Ejecutandose)"
$notifyIcon.Visible = $true

# --- Menú Contextual ---
$contextMenu = New-Object System.Windows.Forms.ContextMenuStrip

# Opción: Abrir Navegador
$itemBrowser = $contextMenu.Items.Add("Abrir en Navegador")
$itemBrowser.Add_Click({
    Start-Process "http://localhost:3000"
})

# Opción: Mostrar Consola
$itemShow = $contextMenu.Items.Add("Mostrar Consola")
$itemShow.Add_Click({
    Show-All
})

# Opción: Ocultar Consola
$itemHide = $contextMenu.Items.Add("Ocultar Consola")
$itemHide.Add_Click({
    Hide-All
})

$contextMenu.Items.Add("-") | Out-Null # Separador

# Opción: Salir
$itemExit = $contextMenu.Items.Add("Salir y Detener Servidor")
$itemExit.Add_Click({
    $notifyIcon.Visible = $false
    # Restaurar ventanas antes de salir
    Show-All
    # Matar procesos
    Stop-Process -Name "cargo" -ErrorAction SilentlyContinue
    Stop-Process -Name "sistema-temperaturas" -ErrorAction SilentlyContinue
    # Cerrar app
    $appContext.ExitThread()
    [Environment]::Exit(0) 
})

$notifyIcon.ContextMenuStrip = $contextMenu

# --- Evento Doble Click ---
$notifyIcon.Add_DoubleClick({
    Start-Process "http://localhost:3000"
})

# --- Ocultar ventanas al inicio ---
Hide-All

# Mostrar globo de notificación
$notifyIcon.ShowBalloonTip(3000, "Sistema de Temperaturas", "El servidor se esta ejecutando en segundo plano.", [System.Windows.Forms.ToolTipIcon]::Info)

# --- Bucle de la aplicación ---
$appContext = New-Object System.Windows.Forms.ApplicationContext
[System.Windows.Forms.Application]::Run($appContext)
