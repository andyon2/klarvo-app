# Klarvo Remote Debug: SSH Server Setup
# Run this ONCE on the test laptop as Administrator.
# After this, the dev machine can SSH in from WSL.
#
# Usage: Right-click PowerShell > "Run as Administrator" > paste:
#   Set-ExecutionPolicy Bypass -Scope Process -Force; & "PATH\setup-ssh-server.ps1"

Write-Host "`n=== Klarvo SSH Server Setup ===" -ForegroundColor Cyan

# Step 1: Install OpenSSH Server (built into Windows 11, just needs enabling)
Write-Host "`n[1/4] Installing OpenSSH Server..." -ForegroundColor Yellow
$feature = Get-WindowsCapability -Online | Where-Object Name -like 'OpenSSH.Server*'
if ($feature.State -eq 'Installed') {
    Write-Host "  Already installed." -ForegroundColor Green
} else {
    Add-WindowsCapability -Online -Name 'OpenSSH.Server~~~~0.0.1.0'
    Write-Host "  Installed." -ForegroundColor Green
}

# Step 2: Start the service and set to auto-start
Write-Host "`n[2/4] Starting SSH service..." -ForegroundColor Yellow
Set-Service -Name sshd -StartupType Automatic
Start-Service sshd
Write-Host "  sshd running and set to auto-start." -ForegroundColor Green

# Step 3: Firewall rule
Write-Host "`n[3/4] Opening firewall port 22..." -ForegroundColor Yellow
$rule = Get-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -ErrorAction SilentlyContinue
if ($rule) {
    Enable-NetFirewallRule -Name 'OpenSSH-Server-In-TCP'
    Write-Host "  Firewall rule already exists, enabled." -ForegroundColor Green
} else {
    New-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -DisplayName 'OpenSSH Server (sshd)' `
        -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22
    Write-Host "  Firewall rule created." -ForegroundColor Green
}

# Step 4: Show connection info
Write-Host "`n[4/4] Connection info:" -ForegroundColor Yellow
$ip = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object {
    $_.InterfaceAlias -match 'Wi-Fi|Ethernet' -and $_.IPAddress -notlike '169.*'
}).IPAddress | Select-Object -First 1
$user = $env:USERNAME

Write-Host ""
Write-Host "  ============================================" -ForegroundColor Cyan
Write-Host "  From WSL on the dev machine, run:" -ForegroundColor Cyan
Write-Host ""
Write-Host "    ssh $user@$ip" -ForegroundColor White
Write-Host ""
Write-Host "  Password: your Windows login password" -ForegroundColor Cyan
Write-Host "  ============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Done! SSH server is running." -ForegroundColor Green
Write-Host ""
