#!/usr/bin/env bash
# harden_db_ports.sh — stop Supabase/Postgres from being reachable off-box.
#
# WHY: Supabase publishes its ports on 0.0.0.0 via Docker, so right now anyone
# on the LAN (and, if the router forwards these ports, the public internet) can
# hit raw Postgres (5432/6543) and the Kong API gateway (8000/8443) directly.
# Verified: a laptop on the same Wi-Fi connected straight to Postgres 5432.
#
# HOW: Docker-published ports bypass the normal INPUT chain (they traverse
# FORWARD → DOCKER-USER), so a plain `ufw deny` does NOTHING. The correct lever
# is the DOCKER-USER chain: drop NEW inbound on the physical NIC only. This
# leaves untouched:
#   • localhost (the cloudflared tunnel reaches Kong via 127.0.0.1) — matind
#     keeps working through supabase-uat.matind.cn
#   • inter-container traffic (docker0 / br-* bridges)
#   • established connections
# The non-Docker python service on 8100 binds 0.0.0.0 directly, so it gets a
# normal INPUT rule.
#
# SAFE: does NOT touch port 22 (SSH). Idempotent. Reversible (see --remove).
#
# Usage:
#   sudo bash harden_db_ports.sh            # apply (postgres only, the must-fix)
#   sudo bash harden_db_ports.sh --all      # also lock Kong 8000/8443 + py 8100
#   sudo bash harden_db_ports.sh --remove   # roll everything back
#   sudo bash harden_db_ports.sh --status   # show current rules
#
# Persist across reboot (Debian/Ubuntu): apt-get install -y iptables-persistent
# then `netfilter-persistent save` (or `iptables-save > /etc/iptables/rules.v4`).

set -euo pipefail

NIC="${NIC:-enp4s0}"                 # external/LAN-facing interface
PG_PORTS="5432,6543"                 # raw Postgres (pooler) — the crown jewel
EXTRA_PORTS="8000,8443"              # Supabase Kong API gateway
PY_PORT="8100"                       # standalone python (not Docker)

have_chain() { iptables -L DOCKER-USER -n >/dev/null 2>&1; }

ensure_docker_user() {
  if ! have_chain; then
    echo "WARN: DOCKER-USER chain missing (is Docker running?). Postgres rules skipped." >&2
    return 1
  fi
}

add_rule() {  # idempotent insert
  local chain="$1"; shift
  if ! iptables -C "$chain" "$@" 2>/dev/null; then
    iptables -I "$chain" "$@"
    echo "  + $chain $*"
  else
    echo "  = $chain $* (already present)"
  fi
}

del_rule() {
  local chain="$1"; shift
  while iptables -C "$chain" "$@" 2>/dev/null; do
    iptables -D "$chain" "$@"
    echo "  - $chain $*"
  done
}

apply() {
  local all="$1"
  echo "Applying DB port hardening on NIC=$NIC ..."
  if ensure_docker_user; then
    add_rule DOCKER-USER -i "$NIC" -p tcp -m conntrack --ctstate NEW \
             -m multiport --dports "$PG_PORTS" -j DROP
    if [[ "$all" == "1" ]]; then
      add_rule DOCKER-USER -i "$NIC" -p tcp -m conntrack --ctstate NEW \
               -m multiport --dports "$EXTRA_PORTS" -j DROP
    fi
  fi
  if [[ "$all" == "1" ]]; then
    add_rule INPUT -i "$NIC" -p tcp --dport "$PY_PORT" -m conntrack --ctstate NEW -j DROP
  fi
  echo "Done. Verify from another LAN host:  nc -z -w3 <server-lan-ip> 5432  (should fail)."
  echo "Persist across reboot:  apt-get install -y iptables-persistent && netfilter-persistent save"
}

remove() {
  echo "Removing DB port hardening ..."
  if have_chain; then
    del_rule DOCKER-USER -i "$NIC" -p tcp -m conntrack --ctstate NEW -m multiport --dports "$PG_PORTS" -j DROP
    del_rule DOCKER-USER -i "$NIC" -p tcp -m conntrack --ctstate NEW -m multiport --dports "$EXTRA_PORTS" -j DROP
  fi
  del_rule INPUT -i "$NIC" -p tcp --dport "$PY_PORT" -m conntrack --ctstate NEW -j DROP
  echo "Done."
}

status() {
  echo "=== DOCKER-USER ==="; iptables -L DOCKER-USER -n -v 2>/dev/null || echo "(no DOCKER-USER chain)"
  echo "=== INPUT (filtered) ==="; iptables -L INPUT -n -v | grep -E "$PY_PORT|Chain" || true
}

case "${1:-}" in
  --remove) remove ;;
  --status) status ;;
  --all)    apply 1 ;;
  "")       apply 0 ;;
  *) echo "unknown arg: $1"; exit 2 ;;
esac
