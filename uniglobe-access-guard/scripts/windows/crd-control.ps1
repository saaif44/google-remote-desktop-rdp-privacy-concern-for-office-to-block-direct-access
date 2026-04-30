param(
  [Parameter(Mandatory=$true)]
  [ValidateSet("status", "enable", "disable", "start", "stop", "allow-once")]
  [string]$Action,
  [int]$Minutes = 30
)

$service = "chromoting"

function Assert-Admin {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = New-Object Security.Principal.WindowsPrincipal($identity)
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Run as Administrator."
  }
}

function Status {
  sc.exe query $service
}

Assert-Admin

switch ($Action) {
  "status" { Status }
  "enable" {
    sc.exe config $service start= auto
    sc.exe start $service
  }
  "disable" {
    sc.exe stop $service
    sc.exe config $service start= disabled
  }
  "start" {
    sc.exe config $service start= demand
    sc.exe start $service
  }
  "stop" {
    sc.exe stop $service
  }
  "allow-once" {
    Write-Host "Access will start in 60 seconds."
    Start-Sleep -Seconds 60
    sc.exe config $service start= demand
    sc.exe start $service
    Start-Sleep -Seconds ($Minutes * 60)
    sc.exe stop $service
    sc.exe config $service start= disabled
  }
}
