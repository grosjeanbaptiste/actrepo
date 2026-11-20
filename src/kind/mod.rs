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
    /// A Cargo crate without axum.
    Crate,
    /// A Dart or npm package.
    Package,
    Tex,
    /// A page printed to PDF, declared by `actmake.toml`.
    Html,
    /// Markdown printed to PDF, declared by `actmake.toml`.
    Markdown,
}

impl Kind {
    /// The actdevice tooling that runs this kind on a device; None for what is not an app.
    pub fn device_tooling(self) -> Option<actdevice::project::Kind> {
        use actdevice::project::Kind as Tooling;
        match self {
            Kind::Flutter => Some(Tooling::Flutter),
            Kind::Hook => Some(Tooling::Hook),
            Kind::Dioxus => Some(Tooling::Dioxus),
            Kind::Api | Kind::Crate | Kind::Package | Kind::Tex | Kind::Html | Kind::Markdown => {
                None
            }
        }
    }

    /// The actmake engine that builds this kind; None until one exists.
    pub fn engine(self) -> Option<Engine> {
        match self {
            Kind::Flutter => Some(Engine::Flutter),
            Kind::Dioxus => Some(Engine::Dioxus),
            Kind::Api | Kind::Crate => Some(Engine::Cargo),
            Kind::Tex => Some(Engine::Tex),
            Kind::Html => Some(Engine::Html),
            Kind::Markdown => Some(Engine::Markdown),
            Kind::Hook | Kind::Package => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum KindError {
    /// The folder exists but no marker file matches.
    Unknown(String),
    /// The folder cannot be read.
    Unreadable(String),
    /// `actmake.toml` names no engine, or one that does not exist.
    BadConfig(String),
}

impl fmt::Display for KindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KindError::Unknown(dir) => write!(f, "cannot tell what kind of project {dir} is"),
            KindError::Unreadable(why) => write!(f, "cannot read the project folder: {why}"),
            KindError::BadConfig(why) => write!(f, "invalid actmake.toml: {why}"),
        }
    }
}

impl std::error::Error for KindError {}

/// The kind of project in `dir`: `actmake.toml` when present (the owner's word), else marker
/// files, first match wins: Flutter, device hook,
/// Dioxus, axum API, LaTeX, Cargo crate, then any other package.
pub fn detect(dir: &Path) -> Result<Kind, KindError> {
    let entries =
        fs::read_dir(dir).map_err(|e| KindError::Unreadable(format!("{}: {e}", dir.display())))?;
    let has_tex = entries
        .filter_map(Result::ok)
        .any(|e| e.path().extension().is_some_and(|x| x == "tex"));
    let text = |name: &str| fs::read_to_string(dir.join(name)).ok();
    if let Some(config) = text("actmake.toml") {
        return declared(&config);
    }
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
    if cargo.is_some() {
        return Ok(Kind::Crate);
    }
    if pubspec.is_some() || dir.join("package.json").is_file() {
        return Ok(Kind::Package);
    }
    Err(KindError::Unknown(dir.display().to_string()))
}

/// The kind an `actmake.toml` declares with `engine = "tex" | "html" | "markdown"`.
fn declared(config: &str) -> Result<Kind, KindError> {
    let value = config
        .lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == "engine")
        .map(|(_, v)| v.trim().trim_matches(|c| c == '"' || c == '\''));
    match value {
        Some("tex") => Ok(Kind::Tex),
        Some("html") => Ok(Kind::Html),
        Some("markdown") => Ok(Kind::Markdown),
        Some(other) => Err(KindError::BadConfig(format!(
            "unknown engine {other:?}: use tex, html or markdown"
        ))),
        None => Err(KindError::BadConfig("no `engine = ...` line".into())),
    }
}

#[cfg(test)]
mod detect_tests;
