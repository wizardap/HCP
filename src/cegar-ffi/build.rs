fn main() {
    println!("cargo:rustc-link-lib=static=cadical");
    println!("cargo:rustc-link-search=native=/work/Cardinality-CDCL/cardinality-cadical/build/");
    // println!("cargo:rustc-link-search=native=/Users/r1/git/Cardinality-CDCL/cardinality-cadical/build/");

    // println!("cargo:rustc-link-lib=c++"); 

    println!("cargo:rustc-link-lib=static=stdc++");
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu/");

    // println!("cargo:rustc-link-lib=dylib=c++"); 
    // println!("cargo:rustc-link-lib=dylib=c++abi");
    // println!("cargo:rustc-link-lib=dylib=stdc++");

    
    // println!("cargo:rustc-link-search=native=../../../../Cardinality-CDCL/cardinality-cadical/build/");
}
