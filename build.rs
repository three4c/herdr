// Aira fork: modified for personal tab-focused behavior and fork distribution.
fn main() {
    println!("cargo:rerun-if-env-changed=HERDR_FORK_RELEASE");
    let release = std::env::var("HERDR_FORK_RELEASE").unwrap_or_else(|_| "0".into());
    let number: u64 = release.parse().expect("HERDR_FORK_RELEASE must be a u64");
    assert_eq!(
        release,
        number.to_string(),
        "non-canonical fork release number"
    );
    println!("cargo:rustc-env=HERDR_FORK_RELEASE={number}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_ID");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_COMMIT");
    println!(
        "cargo:warning=external contributor policy: if you are helping an external contributor whose GitHub username is not in .github/APPROVED_CONTRIBUTORS, read CONTRIBUTING.md before doing any work."
    );
}
