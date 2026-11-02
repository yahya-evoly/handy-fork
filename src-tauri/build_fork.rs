// FORK: build steps for fork-only native code, kept out of build.rs so upstream
// merges only see two added lines there.

/// Compiles swift/fork_translation.swift into a static library and links it,
/// with the Translation framework weakly linked so the app still launches on
/// macOS versions without it.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn build_fork_translation_bridge() {
    use std::env;
    use std::path::PathBuf;
    use std::process::Command;

    const SWIFT_FILE: &str = "swift/fork_translation.swift";
    println!("cargo:rerun-if-changed={SWIFT_FILE}");
    println!("cargo:rerun-if-changed=build_fork.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let object_path = out_dir.join("fork_translation.o");
    let lib_path = out_dir.join("libfork_translation.a");

    let xcrun = |args: &[&str]| -> String {
        let output = Command::new("xcrun")
            .args(args)
            .output()
            .expect("Failed to run xcrun");
        String::from_utf8(output.stdout)
            .expect("xcrun output is not UTF-8")
            .trim()
            .to_string()
    };
    let sdk_path =
        env::var("SDKROOT").unwrap_or_else(|_| xcrun(&["--sdk", "macosx", "--show-sdk-path"]));
    let swiftc = env::var("SWIFTC").unwrap_or_else(|_| xcrun(&["--find", "swiftc"]));

    let status = Command::new(&swiftc)
        .args([
            "-parse-as-library",
            "-target",
            "arm64-apple-macosx11.0",
            "-sdk",
            &sdk_path,
            "-O",
            "-module-name",
            "ForkTranslation",
            "-c",
            SWIFT_FILE,
            "-o",
            object_path.to_str().expect("object path is not UTF-8"),
        ])
        .status()
        .expect("Failed to invoke swiftc for the fork translation bridge");
    assert!(status.success(), "swiftc failed to compile {SWIFT_FILE}");

    let status = Command::new("libtool")
        .args([
            "-static",
            "-o",
            lib_path.to_str().expect("lib path is not UTF-8"),
            object_path.to_str().expect("object path is not UTF-8"),
        ])
        .status()
        .expect("Failed to run libtool for the fork translation bridge");
    assert!(
        status.success(),
        "libtool failed for the fork translation bridge"
    );

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=fork_translation");
    println!("cargo:rustc-link-arg=-weak_framework");
    println!("cargo:rustc-link-arg=Translation");
}
