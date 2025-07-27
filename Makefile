# This file is only used for the contest submission or simulate a contest environment
# And should not be used for development or testing.
# Everything in this Makefile may be completely removed or replaced if the preliminary contest finished.

BUILD_ARTIFACT := bakaos
QEMU_MEM := 1G
QEMU_SMP := 1 # TODO: Fix this

# The judge will use this Makefile to build the kernel and prepare the image for submission.
# And it doesn't like non-ascii characters in the output. so we just completely disable the color output.
LOG ?= OFF
MODE ?= release
ARCH :=
PROFILE ?= virt

ifeq ($(ARCH), riscv64)
	TARGET := riscv64gc-unknown-none-elf
	KERNEL_OUTPUT := kernel-rv
	SDCARD_IMAGE := sdcard-rv.img
else ifeq ($(ARCH), loongarch64)
	TARGET := loongarch64-unknown-none
	KERNEL_OUTPUT := kernel-la
	SDCARD_IMAGE := sdcard-la.img
else ifeq ($(ARCH), )
# forgiving empty
else
$(error Unsupported architecture "${ARCH}". Avaliables are "loongarch64" and "riscv64")
endif

KERNEL_ELF = kernel/target/${TARGET}/${MODE}/${BUILD_ARTIFACT}
QEMU := qemu-system-$(ARCH)

QEMU_ARGS-common := -machine virt \
					-nographic \
					-kernel ${KERNEL_OUTPUT} \
					-m ${QEMU_MEM} \
					-smp ${QEMU_SMP} \
					-rtc base=utc \
					-no-reboot \
					-drive file=${SDCARD_IMAGE},if=none,format=raw,id=x0

QEMU_ARGS-riscv64 := ${QEMU_ARGS-common} \
					-bios default \
					-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

QEMU_ARGS-loongarch64 := ${QEMU_ARGS-common} \
					-device virtio-blk-pci,drive=x0 \

all: _build_all

_build_all:
	@echo "Building for RISC-V64"
	@make build-final ARCH=riscv64
	@echo "Building for LoongArch64"
	@make build-final ARCH=loongarch64

_warn:
	@echo "This Makefile is only used for the contest submission or simulate a contest environment."
	@echo "The all target will only build the kernel and prepare the image for submission."
	@echo ""
	@echo "Starting with environment variables:"
	@echo "ARCH : ${ARCH}"
	@echo "MODE : ${MODE}"
	@echo "LOG  : ${LOG}"

build: _build_internal _prepare_image

_build_internal:
	@echo "Building..."
	cd kernel && LOG=${LOG} cargo build --release --target ${TARGET} --features ${PROFILE} --no-default-features

_prepare_image:
	@echo "Preparing image..."
	@cp ${KERNEL_ELF} ${KERNEL_OUTPUT}
# @rust-objcopy ${KERNEL_OUTPUT} --strip-all -O binary ${KERNEL_OUTPUT}

test: test-only parse

test-only: build _prepare_sdcard _test_internal

_prepare_sdcard:
	@echo "Preparing sdcard..."
	@python3 prepare-img.py ${SDCARD_IMAGE}

_test_internal:
	@echo -e "\e[32m// =========================================\e[0m"
	@echo -e "\e[32m// Starting QEMU output\e[0m"
	@echo -e "\e[32m// =========================================\e[0m"
	@$(QEMU) $(QEMU_ARGS-$(ARCH)) | tee output.log
	@echo -e "\e[32m// =========================================\e[0m"
	@echo -e "\e[32m// QEMU output exited\e[0m"
	@echo -e "\e[32m// =========================================\e[0m"

build-final:
	@KERNEL_TEST="F" make _build_internal
	make _prepare_image

test-final: build-final _prepare_sdcard
	@KERNEL_TEST="F" make _test_final_internal

build-online-final:
	@KERNEL_TEST="O" make _build_internal
	make _prepare_image

test-online-final: build-online-final _prepare_sdcard
	@KERNEL_TEST="O" make _test_final_internal

_test_final_internal: _build_internal _test_internal

vf2:
	@make build ARCH=riscv64 PROFILE=vf2
	@mv kernel-rv kernel-vf2
	@echo "Build complete. Kernel image is in kernel-vf2"

parse:
	@echo "Parsing test output..."
	@python3 test_preliminary/filter_log.py output.log basic.log
	@python3 -W ignore test_preliminary/grading_scripts/test_runner.py basic.log > results.json 2>/dev/null
	@rm basic.log
	@dotnet run --project KernelAnnotationBot -- -f=output.log -b=results.json

clean:
	@echo "Warn: This only cleans files generated for contest submission."
	@rm -f kernel-la kernel-rv sdcard-rv.img sdcard-la.img output.log results.json || exit 0
