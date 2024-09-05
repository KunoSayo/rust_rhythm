fn android_sth() {
    println!("cargo:rustc-link-lib=c++_shared");
}

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or("".to_string()) == "android" {
        android_sth();
    }
}