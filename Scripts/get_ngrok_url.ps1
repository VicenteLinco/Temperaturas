# get_ngrok_url.ps1
$response = try {
    Invoke-RestMethod -Uri 'http://localhost:4040/api/tunnels' -ErrorAction SilentlyContinue
} catch {
    $null
}

if ($response) {
    # Asegurarse de que hay al menos un túnel
    if ($response.tunnels.Count -gt 0) {
        $url = $response.tunnels[0].public_url
        Write-Host ''
        Write-Host 'URL PUBLICA: ' -NoNewline -ForegroundColor Green
        Write-Host $url -ForegroundColor Cyan
        Write-Host ''
        Write-Host 'Servidor local: http://localhost:3000' -ForegroundColor Gray
        Write-Host ''
    } else {
        Write-Host 'Ngrok está corriendo, pero no se encontraron túneles. Revisa la consola de ngrok.' -ForegroundColor Yellow
    }
} else {
    Write-Host 'Error: No se pudo conectar con la API de ngrok en http://localhost:4040.' -ForegroundColor Red
    Write-Host 'Asegúrate de que ngrok se haya iniciado correctamente.' -ForegroundColor Red
}
