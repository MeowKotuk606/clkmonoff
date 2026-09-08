use std::env;

fn main() {
    let out_dir = &env::var("OUT_DIR").unwrap();
    #[cfg(target_os = "linux")] {
        println!("cargo:rerun-if-changed=sub/linux_km");

        use std::fs::{self, File};
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use tar::Builder;
        use sha2::{Sha256, Digest};

        let path = format!("{}/src.tar.gz", out_dir);
        let mut tar = Builder::new(GzEncoder::new(File::create(&path).unwrap(), Compression::default()));
        tar.append_dir_all(".", "sub/linux_km").unwrap();
        tar.into_inner().unwrap().finish().unwrap();
        let hash: String = Sha256::digest(&fs::read(&path).unwrap()).iter().map(|b| format!("{:02x}", b)).collect();
        println!("cargo:rustc-env=SRC_HASH={}", hash);
    }
}