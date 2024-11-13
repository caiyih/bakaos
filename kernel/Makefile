# Region // Build
OUTPUT_NAME = bakaos

CARGO := cargo
CARGO_FLAGS :=
MODE := debug
TARGET := riscv64gc-unknown-none-elf
KERNEL_ELF := target/$(TARGET)/$(MODE)/$(OUTPUT_NAME)

OBJCOPY := rust-objcopy --binary-architecture=riscv64gc

ifeq ($(MODE), release)
	CARGO_FLAGS += --release
endif

# End Region // Build

# Region // Debugging
ARCH := riscv64
GDB := $(ARCH)-elf-gdb
# End Region // Debugging

# Region // Board
QEMU := qemu-system-$(ARCH)

# alternative : 'opensbi' and 'rustsbi' 
SBI_IMPL := opensbi
BOOTLOADER := binary/$(SBI_IMPL).bin

KERNEL_ENTRY_PA := 0x80200000

QEMU_ARGS := -machine virt \
			 -nographic \
			 -bios $(BOOTLOADER) \
			 -kernel $(KERNEL_ELF) \
			 -m 128M
# End Region // Board

all: build

build:
	$(CARGO) build $(CARGO_FLAGS)

strip:
	@$(OBJCOPY) --strip-all $(KERNEL_ELF) $(KERNEL_ELF)

run: build strip
	$(QEMU) $(QEMU_ARGS)

debug: build
	pkill qemu || echo '[!] Previous Qemu killed'
	@echo '[!] Waiting for debugger'
	@$(QEMU) $(QEMU_ARGS) -s -S

gdb: connect

connect:
	@$(GDB) -ex 'add-symbol-file $(KERNEL_ELF) 0xffffffc80200000' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234' \
		-ex 'layout src'

help:
	@echo '- make build   : Build os kernel'
	@echo '- make run     : Run os kernel with qemu'
	@echo '- make debug   : Launch the os with qemu and wait for gdb'
	@echo '- make connect : Launch gdb and connect to qemu'
