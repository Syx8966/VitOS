.PHONY: all kernel-rv kernel-la check-stage0 setup-deps oscomp-kernel-rv oscomp-kernel-rv-run clean

ARCEOS_DIR := third_party/arceos
APP := $(CURDIR)/apps/oscomp-kernel
APP_NAME := oscomp-kernel
RV_OUT_CONFIG := $(CURDIR)/.axconfig-rv.toml
LA_OUT_CONFIG := $(CURDIR)/.axconfig-la.toml
RV_TARGET_DIR := $(CURDIR)/target-rv
LA_TARGET_DIR := $(CURDIR)/target-la
RV_ELF := $(APP)/$(APP_NAME)_riscv64-qemu-virt.elf
LA_ELF := $(APP)/$(APP_NAME)_loongarch64-qemu-virt.elf
RV_BIN := $(APP)/$(APP_NAME)_riscv64-qemu-virt.bin
LA_BIN := $(APP)/$(APP_NAME)_loongarch64-qemu-virt.bin

all: kernel-rv kernel-la

kernel-rv:
	$(MAKE) -C "$(ARCEOS_DIR)" A=$(APP) ARCH=riscv64 LOG=info OUT_CONFIG="$(RV_OUT_CONFIG)" TARGET_DIR="$(RV_TARGET_DIR)"
	cp "$(RV_ELF)" "$@"
	cp "$(RV_BIN)" "$@.bin"

kernel-la:
	$(MAKE) -C "$(ARCEOS_DIR)" A=$(APP) ARCH=loongarch64 LOG=info OUT_CONFIG="$(LA_OUT_CONFIG)" TARGET_DIR="$(LA_TARGET_DIR)"
	cp "$(LA_ELF)" "$@"
	cp "$(LA_BIN)" "$@.bin"

check-stage0:
	@test -d "$(ARCEOS_DIR)" || { echo "missing $(ARCEOS_DIR)"; exit 1; }
	@test -d "third_party/reference/starry-next" || { echo "missing third_party/reference/starry-next"; exit 1; }
	@git -C "$(ARCEOS_DIR)" rev-parse HEAD

setup-deps:
	rustup toolchain install nightly-2025-05-20
	cargo install cargo-axplat axconfig-gen cargo-binutils --locked

oscomp-kernel-rv:
	$(MAKE) -C "$(ARCEOS_DIR)" A=$(APP) ARCH=riscv64 LOG=info OUT_CONFIG="$(RV_OUT_CONFIG)" TARGET_DIR="$(RV_TARGET_DIR)"

oscomp-kernel-rv-run:
	$(MAKE) -C "$(ARCEOS_DIR)" A=$(APP) ARCH=riscv64 LOG=info OUT_CONFIG="$(RV_OUT_CONFIG)" TARGET_DIR="$(RV_TARGET_DIR)" run

clean:
	$(MAKE) -C "$(ARCEOS_DIR)" clean
	rm -f kernel-rv kernel-la kernel-rv.bin kernel-la.bin
