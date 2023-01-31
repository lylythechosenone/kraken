build:
ifeq ($(TARGET),aarch64)
	$(MAKE) build_aarch64
else
	$(error No/invalid target specified. Supported targets are: aarch64)
endif

build_aarch64:
ifeq ($(PROTO), linux)
ifeq ($(DEBUG), 1)
	env RUSTFLAGS="-Clink-arg=-Tsrc/arch/init/aarch64_linux/linker.ld" cargo build --target src/arch/init/aarch64_linux/target.json --features linux -Z build-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem
	cp target/aarch64-unknown-none-softfloat/debug/kraken ./kernel
	objcopy -O binary kernel kernel.bin
else
	env RUSTFLAGS="-Clink-arg=-Tsrc/arch/init/aarch64_linux/linker.ld" cargo build --target aarch64-unknown-none-softfloat --features linux --release
	cp target/aarch64-unknown-none-softfloat/release/kraken ./kernel
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
	qemu-system-aarch64 -M virt -cpu cortex-a76 -m 4096 -device VGA -kernel kernel.bin -serial mon:stdio -s -S
else
	qemu-system-aarch64 -M virt -cpu cortex-a76 -m 4096 -device VGA -kernel kernel -serial mon:stdio
endif
else
	$(error No/invalid protocol specified. Supported aarch64 protocols are: linux)
endif

clean:
	cargo clean
	rm -f kernel
	rm -f kernel.bin
