use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const USER_RUST_TOOLCHAIN: &str = "nightly-2025-05-20";

fn main() {
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_dir = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("kernel crate should live under crates/kernel");
    let source = repo_dir.join("user/hello/src/main.rs");
    let local_basic_source = repo_dir.join("user/local-basic/src/main.rs");
    let linker_script = repo_dir.join("user/hello/linker.ld");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", local_basic_source.display());
    println!("cargo:rerun-if-changed={}", linker_script.display());

    let rv_elf = out_dir.join("hello-rv64.elf");
    let la_elf = out_dir.join("hello-la64.elf");
    let local_basic_rv_elf = out_dir.join("local-basic-rv64.elf");
    let local_basic_la_elf = out_dir.join("local-basic-la64.elf");

    build_user_elf(
        "riscv64gc-unknown-none-elf",
        &source,
        &linker_script,
        &rv_elf,
    );
    build_user_elf(
        "loongarch64-unknown-none-softfloat",
        &source,
        &linker_script,
        &la_elf,
    );
    build_user_elf(
        "riscv64gc-unknown-none-elf",
        &local_basic_source,
        &linker_script,
        &local_basic_rv_elf,
    );
    build_user_elf(
        "loongarch64-unknown-none-softfloat",
        &local_basic_source,
        &linker_script,
        &local_basic_la_elf,
    );

    if target.starts_with("loongarch64") {
        println!("cargo:rustc-env=VITOS_BOOT_ARCH=loongarch64");
    } else if target.starts_with("riscv64") {
        println!("cargo:rustc-env=VITOS_BOOT_ARCH=riscv64");
    }
    println!("cargo:rustc-env=VITOS_USER_HELLO_RV={}", rv_elf.display());
    println!("cargo:rustc-env=VITOS_USER_HELLO_LA={}", la_elf.display());
    println!(
        "cargo:rustc-env=VITOS_LOCAL_BASIC_RV={}",
        local_basic_rv_elf.display()
    );
    println!(
        "cargo:rustc-env=VITOS_LOCAL_BASIC_LA={}",
        local_basic_la_elf.display()
    );
}

fn build_user_elf(target: &str, source: &Path, linker_script: &Path, output: &Path) {
    let status = Command::new("rustup")
        .arg("run")
        .arg(USER_RUST_TOOLCHAIN)
        .arg("rustc")
        .arg("--edition=2024")
        .arg("--target")
        .arg(target)
        .arg(source)
        .arg("-C")
        .arg("panic=abort")
        .arg("-C")
        .arg("relocation-model=static")
        .arg("-C")
        .arg("linker=rust-lld")
        .arg("-C")
        .arg(format!("link-arg=-T{}", linker_script.display()))
        .arg("-C")
        .arg("link-arg=-e")
        .arg("-C")
        .arg("link-arg=_start")
        .arg("-C")
        .arg("link-arg=-z")
        .arg("-C")
        .arg("link-arg=max-page-size=4096")
        .arg("-o")
        .arg(output)
        .stdin(Stdio::null())
        .status()
        .unwrap_or_else(|err| panic!("failed to run rustup/rustc for {target}: {err}"));

    if !status.success() {
        panic!("failed to build user hello ELF for {target}");
    }
}
