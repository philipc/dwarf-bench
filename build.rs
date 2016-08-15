fn main() {
    if std::env::var("CARGO_FEATURE_LIBDWARF").is_ok() {
        println!("cargo:rustc-link-lib=static=dwarf");
    }
    if std::env::var("CARGO_FEATURE_ELFUTILS").is_ok() {
        println!("cargo:rustc-link-lib=dylib=dw");
    }
    println!("cargo:rustc-link-lib=dylib=elf");
    println!("cargo:rustc-link-lib=dylib=z");
    println!("cargo:rustc-link-search=native=/usr/local/lib");
}
