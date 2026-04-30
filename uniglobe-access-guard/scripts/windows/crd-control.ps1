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
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -AssemblyName System.Drawing

    $form = New-Object System.Windows.Forms.Form
    $form.Text = "Remote Access Request"
    $form.Size = New-Object System.Drawing.Size(360, 150)
    $form.StartPosition = "CenterScreen"
    $form.TopMost = $true
    $form.FormBorderStyle = "FixedDialog"
    $form.MaximizeBox = $false
    $form.MinimizeBox = $false
    $form.Tag = 60

    $label = New-Object System.Windows.Forms.Label
    $label.Location = New-Object System.Drawing.Point(20, 20)
    $label.Size = New-Object System.Drawing.Size(300, 40)
    $label.Text = "Incoming remote desktop connection.`nAccess will start in 60 seconds."
    $form.Controls.Add($label)

    $btnGrant = New-Object System.Windows.Forms.Button
    $btnGrant.Location = New-Object System.Drawing.Point(80, 70)
    $btnGrant.Size = New-Object System.Drawing.Size(80, 30)
    $btnGrant.Text = "Grant"
    $btnGrant.DialogResult = "OK"
    $form.Controls.Add($btnGrant)

    $btnDeny = New-Object System.Windows.Forms.Button
    $btnDeny.Location = New-Object System.Drawing.Point(180, 70)
    $btnDeny.Size = New-Object System.Drawing.Size(80, 30)
    $btnDeny.Text = "Revoke"
    $btnDeny.DialogResult = "Cancel"
    $form.Controls.Add($btnDeny)

    $form.AcceptButton = $btnGrant
    $form.CancelButton = $btnDeny

    $timer = New-Object System.Windows.Forms.Timer
    $timer.Interval = 1000
    $timer.Add_Tick({
        $form.Tag--
        $label.Text = "Incoming remote desktop connection.`nAccess will start in $($form.Tag) seconds."
        if ($form.Tag -le 0) {
            $timer.Stop()
            $form.DialogResult = "OK"
            $form.Close()
        }
    })

    $timer.Start()
    $result = $form.ShowDialog()
    $timer.Stop()
    $form.Dispose()

    if ($result -eq "OK") {
        Write-Host "Access granted. Starting for $Minutes minutes."
        sc.exe config $service start= demand
        sc.exe start $service
        Start-Sleep -Seconds ($Minutes * 60)
        sc.exe stop $service
        sc.exe config $service start= disabled
    } else {
        Write-Host "Access revoked by user."
    }
  }
}
