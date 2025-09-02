OUTPUT = bakaos-ex
ARCH :=

QEMU :=

ifeq ($(ARCH), riscv64)
	TARGET := riscv64gc-unknown-none-elf
	QEMU := qemu-system-riscv64
else ifeq ($(ARCH), loongarch64)
	TARGET:= loongarch64-unknown-none
	QEMU := qemu-system-loongarch64
else
$(error "Please specify a valid architecture like `make build ARCH=<arch>` where `<arch>` must be riscv64 or loongarch64")
endif

build:
	cd kernel && cargo build -Z build-std=core,alloc --target $(TARGET)
	cp kernel/target/$(TARGET)/debug/$(OUTPUT) kernel-$(ARCH).bin

run: build
	@$(QEMU) \
		-machine virt \
		-nographic \
		-no-reboot \
		-smp 1 \
		-m 1G \
		-kernel kernel-$(ARCH).bin

