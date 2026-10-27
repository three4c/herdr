// Aira fork: modified for personal tab-focused behavior and fork distribution.
//! Build identity helpers.

pub const BASE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn channel() -> &'static str {
    if is_fork() {
        return "aira";
    }
    non_empty(option_env!("HERDR_BUILD_CHANNEL")).unwrap_or("stable")
}

pub fn build_id() -> Option<&'static str> {
    non_empty(option_env!("HERDR_BUILD_ID"))
}

pub fn version() -> String {
    if is_fork() {
        return format!("{BASE_VERSION}-aira.{}", fork_release());
    }
    match channel() {
        "stable" => BASE_VERSION.to_string(),
        channel => match build_id() {
            Some(build_id) => format!("{BASE_VERSION}-{channel}.{build_id}"),
            None => format!("{BASE_VERSION}-{channel}"),
        },
    }
}

/// This source tree always builds the personal fork, including local builds.
pub fn is_fork() -> bool {
    true
}

/// Installed command name; configuration and session namespaces stay shared.
pub fn executable_name() -> &'static str {
    if is_fork() {
        "herdr-aira"
    } else {
        "herdr"
    }
}

pub fn fork_release() -> u64 {
    env!("HERDR_FORK_RELEASE")
        .parse()
        .expect("build.rs validated HERDR_FORK_RELEASE")
}

pub fn is_preview() -> bool {
    channel() == "preview"
}

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn stable_version_defaults_to_cargo_version() {
        assert!(!super::version().is_empty());
    }
}
