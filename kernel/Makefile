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
			 -m 128M \
			 -drive file=../sdcard.img,if=none,format=raw,id=x0 \
             -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
             -device virtio-net-device,netdev=net \
             -netdev user,id=net \
# End Region // Board

all: build

build:
	$(CARGO) build $(CARGO_FLAGS)

strip:
	@$(OBJCOPY) --strip-all $(KERNEL_ELF) $(KERNEL_ELF)

_prepare_sdcard:
	make -C .. _prepare_sdcard

run: build _prepare_sdcard
	$(QEMU) $(QEMU_ARGS) 2>&1 | tee qemu_output.log
	@python3 unwinder.py --silent --file qemu_output.log

debug: build _prepare_sdcard
	pkill qemu || echo '[!] Previous Qemu killed'
	@echo '[!] Waiting for debugger'
	@$(QEMU) $(QEMU_ARGS) -s -S 2>&1 | tee qemu_output.log
	@python3 unwinder.py --silent --file qemu_output.log

stacktrace:
	@python3 unwinder.py

symboltable:
	@echo "Building symbol table of current elf file"
	@riscv64-elf-objdump \
		--line-numbers \
		--demangle=rust \
		--prefix-addresses \
		--start-address=0xffffffc080200000 \
		--disassemble \
		--section=.text \
		$(KERNEL_ELF) > .disassembled

gdb: connect

connect:
	@$(GDB) -ex 'file $(KERNEL_ELF)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234' \
		-ex 'layout src'

help:
	@echo '- make build   : Build os kernel'
	@echo '- make run     : Run os kernel with qemu'
	@echo '- make debug   : Launch the os with qemu and wait for gdb'
	@echo '- make connect : Launch gdb and connect to qemu'
