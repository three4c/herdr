//! Distribution contract for the three4c/herdr personal fork.

use std::collections::BTreeMap;

use serde::Deserialize;

pub const REPOSITORY: &str = "three4c/herdr";
pub const MANIFEST_URL: &str =
    "https://github.com/three4c/herdr/releases/latest/download/fork-latest.json";
pub const TARGETS: [&str; 4] = [
    "linux-x86_64",
    "linux-aarch64",
    "macos-x86_64",
    "macos-aarch64",
];

pub fn parse_version(identity: &str) -> Option<(crate::update::Version, u64)> {
    let (base, number) = identity.split_once("-aira.")?;
    let release: u64 = number.parse().ok()?;
    if release.to_string() != number {
        return None;
    }
    Some((crate::update::Version::parse(base)?, release))
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub distribution: String,
    pub release: u64,
    pub version: String,
    pub base_version: String,
    pub commit: String,
    pub protocol: u32,
    pub endpoint_generation: u32,
    pub notes: String,
    pub assets: BTreeMap<String, String>,
    pub sha256: BTreeMap<String, String>,
}

impl Manifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.distribution != REPOSITORY || self.release == 0 {
            return Err("invalid three4c/herdr distribution or release number".into());
        }
        if crate::update::Version::parse(&self.base_version).is_none()
            || self.version != format!("{}-aira.{}", self.base_version, self.release)
            || self.notes.trim().is_empty()
            || self.commit.len() != 40
            || !self.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("invalid fork release identity or metadata".into());
        }
        for target in TARGETS {
            self.asset(target)?;
        }
        Ok(())
    }

    pub fn asset(&self, target: &str) -> Result<(&str, &str), String> {
        if !TARGETS.contains(&target) {
            return Err(format!("unsupported fork target: {target}"));
        }
        let expected = format!(
            "https://github.com/{REPOSITORY}/releases/download/aira-v{}/herdr-{target}",
            self.release
        );
        let url = self.assets.get(target).ok_or("missing fork binary")?;
        let checksum = self.sha256.get(target).ok_or("missing fork checksum")?;
        if url != &expected {
            return Err("fork binary URL must belong to the matching three4c/herdr release".into());
        }
        if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("invalid fork SHA-256 checksum".into());
        }
        Ok((url, checksum))
    }
}

#[cfg(test)]
pub(crate) fn test_manifest(release: u64) -> Manifest {
    Manifest {
        distribution: REPOSITORY.into(),
        release,
        version: format!("0.9.5-aira.{release}"),
        base_version: "0.9.5".into(),
        commit: "a".repeat(40),
        protocol: crate::protocol::PROTOCOL_VERSION,
        endpoint_generation: crate::protocol::endpoint::ENDPOINT_PROTOCOL_GENERATION,
        notes: "Fork release notes".into(),
        assets: TARGETS
            .into_iter()
            .map(|target| {
                (
                    target.into(),
                    format!("https://github.com/{REPOSITORY}/releases/download/aira-v{release}/herdr-{target}"),
                )
            })
            .collect(),
        sha256: TARGETS
            .into_iter()
            .map(|target| (target.into(), "a".repeat(64)))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_upstream_assets_and_invalid_integrity_metadata() {
        for target in TARGETS {
            let mut manifest = test_manifest(2);
            assert!(manifest.validate().is_ok());
            manifest
                .assets
                .insert(target.into(), "https://herdr.dev/latest.json".into());
            assert!(manifest.validate().is_err());
            let mut manifest = test_manifest(2);
            manifest.sha256.insert(target.into(), "invalid".into());
            assert!(manifest.validate().is_err());
        }
        assert!(test_manifest(0).validate().is_err());
        let mut manifest = test_manifest(2);
        manifest.distribution = "herdrdev/herdr".into();
        assert!(manifest.validate().is_err());
    }
}
