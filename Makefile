# This file is only used for the contest submission or simulate a contest environment
# And should not be used for development or testing.
# Everything in this Makefile may be completely removed or replaced if the preliminary contest finished.

KERNEL_ELF := kernel-qemu
SBI_OUTPUT := sbi-qemu
ARCH := riscv64gc-unknown-none-elf
# The judge will use this Makefile to build the kernel and prepare the image for submission.
# And it doesn't like non-ascii characters in the output. so we just completely disable the color output.
LOG ?= OFF

SDCARD_IMAGE =./sdcard.img
PRELIMINARY_SDCARD_IMAGE = test_preliminary/sdcard.img
HASH_TO_USE = xxh128sum

all: _warn build

_warn:
	@echo "This Makefile is only used for the contest submission or simulate a contest environment."
	@echo "The all target will only build the kernel and prepare the image for submission."

build: _build_internal _prepare_image

_build_internal:
	@echo "Building..."
	@cd kernel && LOG=$(LOG) cargo build --release

_prepare_image:
	@echo "Preparing image..."
	@cp kernel/target/${ARCH}/release/bakaos ${KERNEL_ELF}
	@rust-objcopy ${KERNEL_ELF} --strip-all -O binary ${KERNEL_ELF}
	@cp kernel/binary/opensbi.bin ${SBI_OUTPUT}

test: test-only parse

test-only: build _prepare_sdcard _test_internal

_prepare_sdcard:
	@echo "Preparing sdcard..."
# Only copy sdcard.img if not exists or modified
	@if command -v $(HASH_TO_USE) &> /dev/null; then \
		if [ -f $(SDCARD_IMAGE) ]; then \
			HASH1=$$($(HASH_TO_USE) $(SDCARD_IMAGE) | cut -d' ' -f1); \
			HASH2=$$($(HASH_TO_USE) $(PRELIMINARY_SDCARD_IMAGE) | cut -d' ' -f1); \
			if [ "$$HASH1" != "$$HASH2" ]; then \
				echo "Hash values are different. Copying from test_preliminary..."; \
				cp $(PRELIMINARY_SDCARD_IMAGE) $(SDCARD_IMAGE); \
			fi; \
		else \
			echo "sdcard.img does not exist. Copying from test_preliminary..."; \
			cp $(PRELIMINARY_SDCARD_IMAGE) $(SDCARD_IMAGE); \
		fi \
	else \
		echo "'$(HASH_TO_USE)' does not exist. Copying sdcard.img..."; \
		cp $(PRELIMINARY_SDCARD_IMAGE) $(SDCARD_IMAGE); \
	fi

_test_internal:
	@qemu-system-riscv64 -machine virt \
        -m 128M -nographic -smp 2 \
        -kernel kernel-qemu \
        -bios sbi-qemu \
        -drive file=sdcard.img,if=none,format=raw,id=x0 \
        -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
        -device virtio-net-device,netdev=net \
        -netdev user,id=net | tee output.log

build-final:
	@KERNEL_TEST="F" make build

test-final: build
	@KERNEL_TEST="F" make _test_final_internal

_test_final_internal: build _test_internal

parse:
	@echo "Parsing test output..."
	@python3 -W ignore test_preliminary/grading_scripts/test_runner.py output.log > results.json
	@echo "Visualizing test results..."
	@python3 test_preliminary/visualize_result.py results.json

clean:
	@echo "Warn: This only cleans files generated for contest submission."
	@rm -f ${KERNEL_ELF} ${SBI_OUTPUT} sdcard.img output.log results.json || exit 0
