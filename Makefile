.PHONY: all check-stage0 setup-deps arceos-helloworld-rv arceos-helloworld-rv-run clean

ARCEOS_DIR := third_party/arceos

all:
	@echo "stage 0: kernel-rv/kernel-la build is not wired yet"
	@echo "next: choose the minimal ArceOS app and connect this target to produce kernel-rv and kernel-la"
	@exit 1

check-stage0:
	@test -d "$(ARCEOS_DIR)" || { echo "missing $(ARCEOS_DIR)"; exit 1; }
	@test -d "third_party/reference/starry-next" || { echo "missing third_party/reference/starry-next"; exit 1; }
	@git -C "$(ARCEOS_DIR)" rev-parse HEAD

setup-deps:
	rustup toolchain install nightly-2025-05-20
	cargo install cargo-axplat axconfig-gen cargo-binutils --locked

arceos-helloworld-rv:
	$(MAKE) -C "$(ARCEOS_DIR)" A=examples/helloworld ARCH=riscv64 LOG=info

arceos-helloworld-rv-run:
	$(MAKE) -C "$(ARCEOS_DIR)" A=examples/helloworld ARCH=riscv64 LOG=info run

clean:
	$(MAKE) -C "$(ARCEOS_DIR)" clean
