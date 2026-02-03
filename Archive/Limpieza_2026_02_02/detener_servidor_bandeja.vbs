' Script para detener el servidor de temperaturas que está en la bandeja
Option Explicit

Dim objShell, objWMIService, colProcesses, objProcess
Dim intKilledCount

Set objShell = CreateObject("WScript.Shell")
Set objWMIService = GetObject("winmgmts:\\.\root\cimv2")

intKilledCount = 0

' Detener ngrok
Set colProcesses = objWMIService.ExecQuery("SELECT * FROM Win32_Process WHERE Name = 'ngrok.exe'")
For Each objProcess In colProcesses
    objProcess.Terminate()
    intKilledCount = intKilledCount + 1
Next

' Detener cargo/rust (servidor)
Set colProcesses = objWMIService.ExecQuery("SELECT * FROM Win32_Process WHERE Name LIKE '%cargo%' OR Name LIKE '%rust%' OR CommandLine LIKE '%cargo run%'")
For Each objProcess In colProcesses
    objProcess.Terminate()
    intKilledCount = intKilledCount + 1
Next

' Detener PowerShell de bandeja del sistema
Set colProcesses = objWMIService.ExecQuery("SELECT * FROM Win32_Process WHERE Name = 'powershell.exe' AND CommandLine LIKE '%NotifyIcon%'")
For Each objProcess In colProcesses
    objProcess.Terminate()
    intKilledCount = intKilledCount + 1
Next

If intKilledCount > 0 Then
    MsgBox "Servidor detenido correctamente." & vbCrLf & _
           "Procesos finalizados: " & intKilledCount, vbInformation, "Sistema de Temperaturas"
Else
    MsgBox "No se encontraron procesos del servidor en ejecución.", vbInformation, "Sistema de Temperaturas"
End If

Set objShell = Nothing
Set objWMIService = Nothing
WScript.Quit
