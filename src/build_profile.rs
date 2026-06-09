//! Build profile (strictness level) and mutex-mode mappings.
//!
//! Profiles map to the UBT strict-compilation flag sets documented in
//! `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md`.

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BuildProfile {
    /// Daily dev: catches header/cpp mismatches, +10-20% compile time.
    Light,
    /// PR verification: light + warnings-as-errors.
    Medium,
    /// Pre-merge: full rebuild, no unity, no shared PCH, force UHT regen.
    Heavy,
}

impl BuildProfile {
    pub fn label(self) -> &'static str {
        match self {
            BuildProfile::Light => "light",
            BuildProfile::Medium => "medium",
            BuildProfile::Heavy => "heavy",
        }
    }

    /// UBT flags for this profile. Sources:
    ///   UE_5.5/Engine/Source/Programs/UnrealBuildTool/Configuration/
    ///     BuildConfiguration.cs, TargetDescriptor.cs, TargetRules.cs
    pub fn flags(self) -> &'static [&'static str] {
        match self {
            BuildProfile::Light => &[
                "-FailIfGeneratedCodeChanges",
                "-NoUBTMakefiles",
                "-DisableAdaptiveUnity",
            ],
            BuildProfile::Medium => &[
                "-FailIfGeneratedCodeChanges",
                "-NoUBTMakefiles",
                "-DisableAdaptiveUnity",
                "-WarningsAsErrors",
            ],
            BuildProfile::Heavy => &[
                "-FailIfGeneratedCodeChanges",
                "-ForceHeaderGeneration",
                "-Rebuild",
                "-DisableUnity",
                "-NoSharedPCH",
                "-WarningsAsErrors",
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MutexMode {
    /// Pick WaitMutex when engine intermediate cache is missing,
    /// NoMutex once it is ready. Validator hint biases to NoMutex.
    Auto,
    /// Always queue: safe for first-build / engine-recompile scenarios.
    Wait,
    /// Always run in parallel: fastest when shared PCH is already built.
    NoMutex,
}

impl MutexMode {
    pub fn as_arg(self) -> &'static str {
        match self {
            MutexMode::Wait => "-WaitMutex",
            MutexMode::NoMutex => "-NoMutex",
            // Auto is decided at runtime, never returned as a literal arg.
            MutexMode::Auto => "-NoMutex",
        }
    }
}

/// Resolve the effective MutexMode for a build invocation.
///
/// `validator_hint` represents a non-IDE caller that should not block:
/// - Validator (e.g. CI smoke test) wants to detect the build is busy,
///   not queue for it. So when Auto + validator_hint, prefer NoMutex.
/// - IDE (no hint) keeps the legacy "queue if uncertain" behaviour.
pub fn resolve_mutex(mode: MutexMode, validator_hint: bool, engine_intermediate_ready: bool) -> MutexMode {
    match mode {
        MutexMode::Wait => MutexMode::Wait,
        MutexMode::NoMutex => MutexMode::NoMutex,
        MutexMode::Auto => {
            if validator_hint {
                // Validator path: never queue. If a real build is running,
                // we'll fail fast and the caller can decide.
                MutexMode::NoMutex
            } else if engine_intermediate_ready {
                MutexMode::NoMutex
            } else {
                MutexMode::Wait
            }
        }
    }
}
