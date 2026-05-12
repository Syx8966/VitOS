#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ ! -f kernel-la ]]; then
    echo "missing kernel-la; run: make all" >&2
    exit 1
fi

qemu-system-loongarch64 \
    -kernel kernel-la \
    -m "${MEM:-1G}" \
    -nographic \
    -smp "${SMP:-1}" \
    -no-reboot
