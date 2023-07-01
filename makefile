OBJCOPY ?= objcopy

build:
ifeq ($(TARGET),aarch64)
	$(MAKE) build_aarch64
else
	$(error No/invalid target specified. Supported targets are: aarch64)
endif

build_aarch64:
ifeq ($(PROTO), linux)
ifeq ($(DEBUG), 1)
	echo ". += 0x2000000;" > stack_size.ld
	cargo build --target src/arch/init/aarch64_linux/target.json --features linux -Z build-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem
	cp target/target/debug/kraken ./kernel
	$(OBJCOPY) -O binary kernel kernel.bin
	rm -f stack_size.ld
else
	echo ". += 0x200000;" > stack_size.ld
	cargo build --target src/arch/init/aarch64_linux/target.json --features linux -Z build-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem --release
	cp target/target/release/kraken ./kernel
	$(OBJCOPY) -O binary kernel kernel.bin
	rm -f stack_size.ld
endif
else
	$(error No/invalid protocol specified. Supported aarch64 protocols are: linux)
endif

qemu:
ifeq ($(TARGET),aarch64)
	$(MAKE) qemu_aarch64
else
	$(error No/invalid target specified. Supported targets are: aarch64)
endif

qemu_aarch64: build_aarch64
ifeq ($(PROTO), linux)
ifeq ($(DEBUG), 1)
	qemu-system-aarch64 -M virt -cpu cortex-a76 -m 4096 -vga none -device ramfb -kernel kernel.bin -serial mon:stdio -d int -s -S
else
	qemu-system-aarch64 -M virt -cpu cortex-a76 -m 4096 -vga none -device ramfb -kernel kernel.bin -serial stdio
endif
else
	$(error No/invalid protocol specified. Supported aarch64 protocols are: linux)
endif

clean:
	cargo clean
	rm -f kernel
	rm -f kernel.bin
