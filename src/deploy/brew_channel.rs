//! Homebrew release channel, pure part: what differs between the stable release and the beta.
//! Stable: release `v<version>`, cask `acteble`. Beta: prerelease `v<version>-beta.<build>`, cask `acteble@beta`.
use super::brew_beta::beta_version;
use super::target::Env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
}

impl Channel {
    /// prod publishes the stable release, test the beta.
    pub fn of(env: Env) -> Channel {
        if env == Env::Prod {
            Channel::Stable
        } else {
            Channel::Beta
        }
    }

    /// The `--channel` the cask bumper takes.
    pub fn name(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
        }
    }

    /// Stable: the pubspec version without its `+build`. Beta: `<version>-beta.<build>`.
    pub fn version(
        self,
        base: &str,
        build: impl FnOnce() -> anyhow::Result<u64>,
    ) -> anyhow::Result<String> {
        match self {
            Channel::Stable => Ok(base.split('+').next().unwrap_or(base).to_string()),
            Channel::Beta => Ok(beta_version(base, build()?)),
        }
    }

    /// A prerelease is never the tap's "latest" release; the stable release is.
    pub fn prerelease(self) -> bool {
        self == Channel::Beta
    }

    pub fn title(self, app: &str, version: &str, tag: &str) -> String {
        match self {
            Channel::Stable => format!("{app} {version}"),
            Channel::Beta => tag.to_string(),
        }
    }

    pub fn notes(self) -> &'static str {
        match self {
            Channel::Stable => "Stable release",
            Channel::Beta => "Beta build",
        }
    }

    pub fn commit_message(self, app: &str, version: &str) -> String {
        match self {
            Channel::Stable => format!("chore: {app} {version}"),
            Channel::Beta => format!("chore: {app} beta {version}"),
        }
    }
}

#[cfg(test)]
mod tests {
    include!("brew_channel_tests.rs");
}
