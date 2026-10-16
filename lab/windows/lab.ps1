# SPDX-License-Identifier: GPL-3.0-or-later
# Failure lab (F2-LAB, plan v1 VIII.6): breaks one known thing in a THROWAWAY Windows VM so the app
# can be checked against it, and puts it back. Never run it on a real computer: it takes the
# network down, fills the disk and stops the audio service. Run from an elevated PowerShell.
#
#   $env:HMD_LAB_VM = '1'; .\lab\windows\lab.ps1 -Scenario dns -Action break
param(
    [Parameter(Mandatory)][ValidateSet('dns', 'adapter', 'disk', 'audio', 'startup')][string]$Scenario,
    [Parameter(Mandatory)][ValidateSet('break', 'restore')][string]$Action
)
$ErrorActionPreference = 'Stop'

# Two locks: the person says so, and the machine is a VM.
if ($env:HMD_LAB_VM -ne '1') { throw 'set HMD_LAB_VM=1 to confirm this is a throwaway VM' }
$model = (Get-CimInstance Win32_ComputerSystem).Model
if ($model -notmatch 'Virtual|VMware|VirtualBox|KVM|QEMU|HVM domU') { throw "this machine ($model) is not a VM; refusing" }

$state = Join-Path $env:LOCALAPPDATA 'hmd-lab'
New-Item -ItemType Directory -Force $state | Out-Null
$fill = Join-Path $env:PUBLIC 'hmd-lab-fill.bin'
$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'

function Active-Interface {
    (Get-NetIPConfiguration | Where-Object { $_.IPv4DefaultGateway } | Select-Object -First 1).InterfaceAlias
}

switch ("$Scenario`:$Action") {
    'dns:break' {
        $if = Active-Interface
        if (-not $if) { throw 'no interface with a default gateway' }
        $current = (Get-DnsClientServerAddress -InterfaceAlias $if -AddressFamily IPv4).ServerAddresses
        @{ interface = $if; servers = @($current) } | ConvertTo-Json | Set-Content (Join-Path $state 'dns.json')
        # 192.0.2.53 is TEST-NET-1 (RFC 5737): nothing answers there.
        Set-DnsClientServerAddress -InterfaceAlias $if -ServerAddresses 192.0.2.53
        Clear-DnsClientCache
        'expected finding: net.dns.resolution_failing (family 1)'
    }
    'dns:restore' {
        $saved = Get-Content (Join-Path $state 'dns.json') | ConvertFrom-Json
        # The saved list is what Windows showed, DHCP or not; resetting is right when it came from DHCP.
        Set-DnsClientServerAddress -InterfaceAlias $saved.interface -ResetServerAddresses
        Clear-DnsClientCache
        "restored to automatic DNS; it was: $($saved.servers -join ', ')"
        Remove-Item (Join-Path $state 'dns.json')
    }
    'adapter:break' {
        $if = Active-Interface
        if (-not $if) { throw 'no interface with a default gateway' }
        Set-Content (Join-Path $state 'adapter.txt') $if
        Disable-NetAdapter -Name $if -Confirm:$false
        'expected finding: net.no_active_adapter (family 1)'
    }
    'adapter:restore' {
        Enable-NetAdapter -Name (Get-Content (Join-Path $state 'adapter.txt')) -Confirm:$false
        Remove-Item (Join-Path $state 'adapter.txt')
    }
    'disk:break' {
        # storage.low_space: free below 10% or below 5 GiB. Leave 4 GiB free on the system drive.
        $free = (Get-PSDrive -Name ($env:SystemDrive.TrimEnd(':'))).Free
        $keep = 4GB
        if ($free -le $keep) { throw 'already below 4 GiB free; nothing to do' }
        fsutil file createnew $fill ($free - $keep) | Out-Null
        fsutil file setvaliddata $fill ($free - $keep) | Out-Null
        $after = (Get-PSDrive -Name ($env:SystemDrive.TrimEnd(':'))).Free
        "free space now: $([math]::Round($after / 1GB, 1)) GiB"
        'expected finding: storage.low_space (family 3)'
    }
    'disk:restore' {
        Remove-Item $fill -Force
    }
    'audio:break' {
        Stop-Service -Name Audiosrv -Force
        'expected finding: av.no_output_device (family 4) - to be confirmed in the lab'
    }
    'audio:restore' {
        Start-Service -Name Audiosrv
    }
    'startup:break' {
        # startup.many_apps: 8 enabled entries or more. Eight harmless ones.
        1..8 | ForEach-Object { Set-ItemProperty -Path $run -Name "HMD-Lab-$_" -Value 'cmd.exe /c exit' }
        'expected finding: startup.many_apps (family 2, with the startup-apps consent)'
    }
    'startup:restore' {
        1..8 | ForEach-Object { Remove-ItemProperty -Path $run -Name "HMD-Lab-$_" -ErrorAction SilentlyContinue }
    }
}
