fn main() {
    cc::Build::new()
        .file("src/solver_wrapper.c")
        .compile("solver_wrapper");

    println!("cargo:rustc-link-lib=static=cadical");
    if let Ok(dir) = std::env::var("CADICAL_DIR") {
        println!("cargo:rustc-link-search=native={}", dir);
    }
    println!("cargo:rustc-link-search=native=/work/Cardinality-CDCL/cardinality-cadical/build/");
    println!("cargo:rustc-link-search=native=/home/ubuntu/HCP/src/cegar-fix/target/release/build/rustsat-cadical-8235ddb40f102300/out");

    println!("cargo:rustc-link-lib=static=stdc++");
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu/");
}
