# Region // Build
OUTPUT_NAME = bakaos
CARGO := cargo
OBJCOPY := rust-objcopy

ARCH := riscv64

ifeq ($(ARCH), riscv64)
	GDB := riscv64-elf-gdb
	TARGET := riscv64gc-unknown-none-elf
	OBJDUMP := riscv64-elf-objdump
else ifeq ($(ARCH), loongarch64)
# can be obtained from https://github.com/LoongsonLab/oscomp-toolchains-for-oskernel 
	GDB := loongarch64-linux-gnu-gdb
	TARGET:= loongarch64-unknown-none
	OBJDUMP := loongarch64-linux-gnu-objdump
else
	$(error "Unsupported architecture")
endif

CARGO_FLAGS := --target $(TARGET)

MODE := debug
KERNEL_ELF := target/$(TARGET)/$(MODE)/$(OUTPUT_NAME)

ifneq ($(MODE), debug)
	CARGO_FLAGS += --profile=$(MODE)
endif

QEMU := qemu-system-$(ARCH)

QEMU-ARGS-common := -machine virt \
			 -nographic \

# alternative : 'opensbi' and 'rustsbi' 
SBI_IMPL := opensbi
BOOTLOADER := binary/$(SBI_IMPL).bin

QEMU-ARGS-riscv64 := $(QEMU-ARGS-common) \
			 -bios $(BOOTLOADER) \
			 -kernel $(KERNEL_ELF) \
			 -m 128M \
			 -drive file=../sdcard.img,if=none,format=raw,id=x0 \
			 -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
			 -device virtio-net-device,netdev=net \
			 -netdev user,id=net \

QEMU-ARGS-loongarch64 := $(QEMU-ARGS-common) \
			 -m 1G \
			 -kernel $(KERNEL_ELF) \
			 -no-reboot \

all: help

build:
	$(CARGO) build $(CARGO_FLAGS)

strip:
	@$(OBJCOPY) --strip-all $(KERNEL_ELF) $(KERNEL_ELF)

_prepare_sdcard:
	make -C .. _prepare_sdcard

run: build _prepare_sdcard
	$(QEMU) $(QEMU-ARGS-$(ARCH)) 2>&1 | tee qemu_output.log
	@python3 unwinder.py --silent --file qemu_output.log

debug: build _prepare_sdcard
	pkill qemu || echo '[!] Previous Qemu killed'
	@echo '[!] Waiting for debugger'
	@$(QEMU) $(QEMU-ARGS-$(ARCH)) -s -S 2>&1 | tee qemu_output.log
	@python3 unwinder.py --silent --file qemu_output.log

stacktrace:
	@python3 unwinder.py

symboltable:
	@echo "Building symbol table of current elf file"
	@$(OBJDUMP) \
		--line-numbers \
		--demangle=rust \
		--prefix-addresses \
		--disassemble \
		--section=.text \
		$(KERNEL_ELF) > .disassembled

gdb: connect

connect:
	@$(GDB) -ex 'file $(KERNEL_ELF)' \
		-ex 'target remote localhost:1234' \
		-ex 'layout src'

help:
	@echo 'BakaOS build system'
	@echo 'Usage:'
	@echo '- make help    : Show this help message'
	@echo '- make build   : Build os kernel'
	@echo '- make run     : Run os kernel with qemu'
	@echo '- make debug   : Launch the os with qemu and wait for gdb'
	@echo '- make connect : Launch gdb and connect to qemu'
	@echo '- make clean   : Clean the build artifacts'
	@echo '- make stacktrace : Parse and generate the stacktrace of qemu output'
	@echo '- make symboltable: Generate symbol table of current elf file at kernel/.disassembled'
	@echo '- make strip   : Strip the kernel elf file, usually this is unnecessary'
	@echo ''
	@echo 'Environment variables:'
	@echo '- ARCH=riscv64(default)|loongarch64            : Set the target architecture'
	@echo '- MODE=debug(default)|release-with-debug       : Set the build mode'
	@echo '- LOG=TRACE|DEBUG|INFO(default)|WARN|ERROR|OFF : Set the log level'
