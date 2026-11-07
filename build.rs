fn main() {
    println!("cargo:rerun-if-changed=assets/telysta.ico");
    println!("cargo:rerun-if-changed=assets/windows.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/telysta.ico")
            .set_manifest_file("assets/windows.manifest")
            .set("ProductName", "Telysta")
            .set("FileDescription", "Telysta · 图片成书")
            .set("LegalCopyright", "Telysta contributors · MIT");
        resource
            .compile()
            .expect("Failed to compile Windows resources");
    }
}
