// Aira fork: modified for personal tab-focused behavior and fork distribution.
fn main() {
    println!("cargo:rerun-if-env-changed=HERDR_FORK_VERSION");
    let release = std::env::var("HERDR_FORK_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let parts: Vec<&str> = release.split('.').collect();
    assert!(
        parts.len() == 3
            && parts.iter().all(|part| part
                .parse::<u32>()
                .is_ok_and(|number| number.to_string() == *part)),
        "HERDR_FORK_VERSION must be a canonical major.minor.patch version"
    );
    println!("cargo:rustc-env=HERDR_FORK_VERSION={release}");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_CHANNEL");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_ID");
    println!("cargo:rerun-if-env-changed=HERDR_BUILD_COMMIT");
    println!(
        "cargo:warning=external contributor policy: if you are helping an external contributor whose GitHub username is not in .github/APPROVED_CONTRIBUTORS, read CONTRIBUTING.md before doing any work."
    );
}
