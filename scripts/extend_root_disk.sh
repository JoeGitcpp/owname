#!/usr/bin/env bash
# extend_root_disk.sh — grow / from 100G to 300G using free LVM space (online, no reboot).
# Run ON the server with sudo:
#   sudo bash ~/aborn/scripts/extend_root_disk.sh
#
# Current layout (server-joe):
#   nvme0n1p3  928G LVM PV  →  ubuntu-vg has ~828G free, ubuntu-lv is only 100G
# Services keep running throughout lvextend + resize2fs.

set -euo pipefail

TARGET_SIZE="${1:-300G}"
LV="/dev/ubuntu-vg/ubuntu-lv"

echo "=== Before ==="
df -hT /
sudo vgs -o vg_name,vg_size,vg_free
sudo lvs -o lv_name,lv_size,vg_name

echo ""
echo "=== Extending $LV to $TARGET_SIZE ==="
sudo lvextend -L "$TARGET_SIZE" "$LV"
sudo resize2fs "$LV"

echo ""
echo "=== After ==="
df -hT /
echo "Done. No reboot required."