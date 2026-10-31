use std::{fs::create_dir, path::PathBuf};

use slu_utils::{checksums::CheckSums, signature::sign_minisign};

fn main() {
    let _ = create_dir("gen");

    let mut checksums = CheckSums::new();
    read_folder_recursive(PathBuf::from("static"), &mut |path| {
        checksums.add(&path).unwrap();
    });

    let target_dir = target_dir();
    let sums_path = target_dir.join("SHA256SUMS");
    checksums.write(&sums_path).unwrap();

    println!("cargo:rerun-if-env-changed=KESTREL_PUBKEY");
    println!("cargo:rerun-if-env-changed=KESTREL_DISCORD_ID");
    println!("cargo:rerun-if-env-changed=KESTREL_GITHUB_CLIENT_ID");

    println!("cargo:rerun-if-env-changed=TAURI_SIGNING_PRIVATE_KEY");

    // local release builds go unsigned: without KESTREL_PUBKEY the app skips
    // the signature check and the updater, so a placeholder is enough
    let signed = !cfg!(debug_assertions) && env_set("KESTREL_PUBKEY");
    if signed {
        sign_sha256sums(&sums_path);
    } else {
        if !cfg!(debug_assertions) {
            println!("cargo:warning=unsigned build: set KESTREL_PUBKEY to enable updates");
        }
        std::fs::write(sums_path.with_extension("sig"), "UNSIGNED").unwrap();
    }

    tauri_build::build();
}

fn target_dir() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // see <https://github.com/rust-lang/cargo/issues/5457>
    out_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_folder_recursive<F>(path: PathBuf, cb: &mut F)
where
    F: FnMut(PathBuf),
{
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            read_folder_recursive(path, cb);
        } else {
            cb(path);
        }
    }
}

fn env_set(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| !v.is_empty())
}

fn sign_sha256sums(path: &PathBuf) {
    let key_base64 =
        std::env::var("TAURI_SIGNING_PRIVATE_KEY").expect("TAURI_SIGNING_PRIVATE_KEY missing");
    let password = std::env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD")
        .expect("TAURI_SIGNING_PRIVATE_KEY_PASSWORD missing");

    let data = std::fs::read(path).expect("Failed to read SHA256SUMS file");
    let signature = sign_minisign(&data, &key_base64, password).expect("Failed to sign data");

    let sig_path = path.with_extension("sig");
    std::fs::write(&sig_path, signature).expect("Failed to write signature");
}
