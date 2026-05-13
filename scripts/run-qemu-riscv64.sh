#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ ! -f kernel-rv.bin ]]; then
    echo "missing kernel-rv.bin; run: make all" >&2
    exit 1
fi

drive_args=()
if [[ -n "${TEST_IMG:-}" ]]; then
    drive_args=(-drive "file=${TEST_IMG},if=none,format=raw,id=testdisk" -device virtio-blk-device,drive=testdisk)
fi

qemu-system-riscv64 \
    -machine virt \
    -kernel kernel-rv.bin \
    -m "${MEM:-128M}" \
    -nographic \
    -smp "${SMP:-1}" \
    -bios default \
    "${drive_args[@]}" \
    -no-reboot
