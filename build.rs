fn main() {
    println!("cargo:rustc-link-arg-bin=kraken=--script=src/arch/init/aarch64_linux/linker.ld");
}
