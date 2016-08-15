fn main() {
    println!("cargo:rustc-link-lib=static=dwarf");
    println!("cargo:rustc-link-lib=dylib=elf");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-search=native=/usr/local/lib");
}
