#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Failure lab (F2-LAB, plan v1 VIII.6): breaks one known thing in a THROWAWAY Linux VM so the app
# can be checked against it, and puts it back. Never run it on a real computer: it takes the
# network down, fills the disk and stops the sound server.
#
#   HMD_LAB_VM=1 lab/linux/lab.sh <dns|adapter|disk|audio|startup> <break|restore>
set -euo pipefail

scenario="${1:-}"
action="${2:-}"
state="${XDG_CACHE_HOME:-$HOME/.cache}/hmd-lab"
mkdir -p "$state"

die() { echo "lab: $*" >&2; exit 1; }

# Two locks: the person says so, and the machine is a VM.
[ "${HMD_LAB_VM:-}" = "1" ] || die "set HMD_LAB_VM=1 to confirm this is a throwaway VM"
command -v systemd-detect-virt >/dev/null || die "systemd-detect-virt not found; cannot tell this is a VM"
systemd-detect-virt --vm --quiet || die "this machine is not a VM; refusing"

active_connection() {
  nmcli -t -f NAME,DEVICE connection show --active | awk -F: '$2 != "lo" && $2 != "" { print $1; exit }'
}

case "$scenario:$action" in
  dns:break)
    con=$(active_connection); [ -n "$con" ] || die "no active NetworkManager connection"
    printf '%s\n' "$con" > "$state/dns.connection"
    nmcli -g ipv4.dns,ipv4.ignore-auto-dns connection show "$con" > "$state/dns.ipv4"
    # 192.0.2.53 is TEST-NET-1 (RFC 5737): nothing answers there.
    nmcli connection modify "$con" ipv4.dns 192.0.2.53 ipv4.ignore-auto-dns yes ipv6.ignore-auto-dns yes
    nmcli connection up "$con" >/dev/null
    resolvectl flush-caches 2>/dev/null || true
    echo "expected finding: net.dns.resolution_failing (family 1)"
    ;;
  dns:restore)
    con=$(cat "$state/dns.connection")
    dns=$(sed -n 1p "$state/dns.ipv4"); ignore=$(sed -n 2p "$state/dns.ipv4")
    nmcli connection modify "$con" ipv4.dns "$dns" ipv4.ignore-auto-dns "$ignore" ipv6.ignore-auto-dns no
    nmcli connection up "$con" >/dev/null
    rm -f "$state/dns.connection" "$state/dns.ipv4"
    ;;
  adapter:break)
    nmcli networking off
    echo "expected finding: net.no_active_adapter or net.no_valid_ip (family 1)"
    ;;
  adapter:restore)
    nmcli networking on
    ;;
  disk:break)
    # storage.low_space: free below 10% or below 5 GiB. Leave 4 GiB free on the home volume.
    free=$(df --output=avail -B1 "$HOME" | tail -1)
    keep=$((4 * 1024 * 1024 * 1024))
    [ "$free" -gt "$keep" ] || die "already below 4 GiB free; nothing to do"
    fallocate -l $((free - keep)) "$HOME/hmd-lab-fill.bin"
    echo "expected finding: storage.low_space (family 3)"
    ;;
  disk:restore)
    rm -f "$HOME/hmd-lab-fill.bin"
    ;;
  audio:break)
    systemctl --user stop pipewire.socket pipewire-pulse.socket pipewire pipewire-pulse wireplumber
    echo "expected finding: av.no_sound_server (family 4)"
    ;;
  audio:restore)
    systemctl --user start pipewire.socket pipewire-pulse.socket pipewire pipewire-pulse wireplumber
    ;;
  startup:break)
    # startup.many_apps: 8 enabled entries or more. Eight harmless ones.
    mkdir -p "$HOME/.config/autostart"
    for i in 1 2 3 4 5 6 7 8; do
      printf '[Desktop Entry]\nType=Application\nName=HMD lab %s\nExec=/bin/true\n' "$i" \
        > "$HOME/.config/autostart/hmd-lab-$i.desktop"
    done
    echo "expected finding: startup.many_apps (family 2, with --consent-startup-apps)"
    ;;
  startup:restore)
    rm -f "$HOME"/.config/autostart/hmd-lab-*.desktop
    ;;
  *)
    die "usage: HMD_LAB_VM=1 $0 <dns|adapter|disk|audio|startup> <break|restore>"
    ;;
esac
