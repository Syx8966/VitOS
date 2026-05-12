#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ ! -f kernel-rv.bin ]]; then
    echo "missing kernel-rv.bin; run: make all" >&2
    exit 1
fi

qemu-system-riscv64 \
    -machine virt \
    -kernel kernel-rv.bin \
    -m "${MEM:-128M}" \
    -nographic \
    -smp "${SMP:-1}" \
    -bios default \
    -no-reboot
