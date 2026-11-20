//! What kind of project a folder holds: the one place that knows, so the build engine of
//! actmake (and the device runner of actdevice) is chosen from the same answer.
use actmake::Engine;
use std::fmt;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Flutter,
    /// Ships an executable `scripts/device-install.sh` and builds and installs itself.
    Hook,
    Dioxus,
    /// A Rust crate on axum.
    Api,
    /// Cargo crate without axum, Dart package, npm package.
    Package,
    Tex,
}

impl Kind {
    /// The actmake engine that builds this kind; None until one exists.
    pub fn engine(self) -> Option<Engine> {
        match self {
            Kind::Tex => Some(Engine::Tex),
            Kind::Flutter | Kind::Hook | Kind::Dioxus | Kind::Api | Kind::Package => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum KindError {
    /// The folder exists but no marker file matches.
    Unknown(String),
    /// The folder cannot be read.
    Unreadable(String),
}

impl fmt::Display for KindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KindError::Unknown(dir) => write!(f, "cannot tell what kind of project {dir} is"),
            KindError::Unreadable(why) => write!(f, "cannot read the project folder: {why}"),
        }
    }
}

impl std::error::Error for KindError {}

/// The kind of project in `dir`, by marker files, first match wins: Flutter, device hook,
/// Dioxus, axum API, LaTeX, then any other package.
pub fn detect(dir: &Path) -> Result<Kind, KindError> {
    let entries =
        fs::read_dir(dir).map_err(|e| KindError::Unreadable(format!("{}: {e}", dir.display())))?;
    let has_tex = entries
        .filter_map(Result::ok)
        .any(|e| e.path().extension().is_some_and(|x| x == "tex"));
    let text = |name: &str| fs::read_to_string(dir.join(name)).ok();
    let pubspec = text("pubspec.yaml");
    if pubspec.as_deref().is_some_and(|s| s.contains("flutter")) {
        return Ok(Kind::Flutter);
    }
    let hook = dir.join("scripts/device-install.sh");
    if fs::metadata(&hook).is_ok_and(|m| m.permissions().mode() & 0o111 != 0) {
        return Ok(Kind::Hook);
    }
    if dir.join("Dioxus.toml").is_file() {
        return Ok(Kind::Dioxus);
    }
    let cargo = text("Cargo.toml");
    if cargo.as_deref().is_some_and(|s| s.contains("axum")) {
        return Ok(Kind::Api);
    }
    if has_tex {
        return Ok(Kind::Tex);
    }
    if cargo.is_some() || pubspec.is_some() || dir.join("package.json").is_file() {
        return Ok(Kind::Package);
    }
    Err(KindError::Unknown(dir.display().to_string()))
}

#[cfg(test)]
mod detect_tests;
