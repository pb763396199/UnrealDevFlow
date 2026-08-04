//! Output routing for both humans and machines.
//!
//! Under `--format json` a command must emit exactly one JSON document on
//! stdout. Commands report progress as they work, so those lines are collected
//! rather than printed and end up inside the document's `messages` array —
//! otherwise they would sit in front of the JSON and break every parser.

use crate::cli::OutputFormat;
use serde::Serialize;
use std::cell::RefCell;

thread_local! {
    /// `Some` while the process runs in JSON mode. Holds the progress lines
    /// collected so far.
    static CAPTURED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    /// Set once a command has emitted its document, so the safety net in
    /// `flush_unemitted` knows there is nothing left to say.
    static EMITTED: RefCell<bool> = const { RefCell::new(false) };
}

/// Machine-readable envelope. Every command emits exactly one of these under
/// `--format json`, whether it succeeded or failed.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a, T: Serialize> {
    command: &'a str,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    messages: Vec<String>,
}

/// Decide once, at startup, how this run talks.
pub fn set_format(format: &OutputFormat) {
    if matches!(format, OutputFormat::Json) {
        CAPTURED.with(|captured| *captured.borrow_mut() = Some(Vec::new()));
    }
}

pub fn is_json() -> bool {
    CAPTURED.with(|captured| captured.borrow().is_some())
}

fn take_messages() -> Vec<String> {
    CAPTURED.with(|captured| captured.borrow_mut().take().unwrap_or_default())
}

/// Print a progress line, or collect it when the answer must be JSON.
fn record(line: String) {
    let collected = CAPTURED.with(|captured| match captured.borrow_mut().as_mut() {
        Some(lines) => {
            lines.push(line.clone());
            true
        }
        None => false,
    });
    if !collected {
        println!("{}", line);
    }
}

fn write_envelope<T: Serialize>(envelope: &Envelope<'_, T>) {
    match serde_json::to_string_pretty(envelope) {
        Ok(json) => println!("{}", json),
        Err(error) => eprintln!("JSON serialization error: {}", error),
    }
}

/// A command's final answer. `human` renders the same data for people.
pub fn emit<T: Serialize>(command: &str, data: T, human: impl FnOnce(&T) -> String) {
    EMITTED.with(|emitted| *emitted.borrow_mut() = true);
    if is_json() {
        write_envelope(&Envelope {
            command,
            ok: true,
            data: Some(data),
            error: None,
            messages: take_messages(),
        });
    } else {
        println!("{}", human(&data));
    }
}

/// A command that failed. In JSON mode the error travels inside the document
/// so callers never have to scrape stderr.
pub fn emit_failure(command: &str, error: &str) {
    EMITTED.with(|emitted| *emitted.borrow_mut() = true);
    if is_json() {
        write_envelope(&Envelope::<()> {
            command,
            ok: false,
            data: None,
            error: Some(error.to_string()),
            messages: take_messages(),
        });
    } else {
        eprintln!("✗ {}", error);
    }
}

/// Safety net for commands that have not been migrated to `emit` yet.
///
/// Without it their progress lines would be collected in JSON mode and then
/// silently dropped, so a successful run would print nothing at all.
/// 这条命令自己负责整个 stdout，安全网不要再补一个信封。
///
/// 只给已经有外部消费方的固定契约用（`aw-status` 是 AgentWatcher 在读）。
/// 新命令一律走 `emit`。
pub fn mark_emitted() {
    EMITTED.with(|emitted| *emitted.borrow_mut() = true);
}

pub fn flush_unemitted(command: &str) {
    let already = EMITTED.with(|emitted| *emitted.borrow());
    if already || !is_json() {
        return;
    }
    write_envelope(&Envelope::<()> {
        command,
        ok: true,
        data: None,
        error: None,
        messages: take_messages(),
    });
}

pub fn print_success(message: &str) {
    record(format!("✓ {}", message));
}

pub fn print_error(message: &str) {
    if is_json() {
        record(format!("✗ {}", message));
    } else {
        eprintln!("✗ {}", message);
    }
}

pub fn print_warning(message: &str) {
    record(format!("⚠ {}", message));
}

pub fn print_info(message: &str) {
    record(format!("ℹ {}", message));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Sample {
        name: String,
    }

    fn reset() {
        CAPTURED.with(|captured| *captured.borrow_mut() = None);
        EMITTED.with(|emitted| *emitted.borrow_mut() = false);
    }

    #[test]
    fn json_mode_collects_progress_lines_instead_of_printing_them() {
        reset();
        set_format(&OutputFormat::Json);
        print_info("working");
        print_warning("careful");
        assert!(is_json());
        let collected = CAPTURED.with(|captured| captured.borrow().clone().unwrap());
        assert_eq!(
            collected,
            vec!["ℹ working".to_string(), "⚠ careful".to_string()]
        );
        reset();
    }

    #[test]
    fn human_mode_keeps_nothing_and_stays_out_of_json() {
        reset();
        set_format(&OutputFormat::Human);
        print_info("working");
        assert!(!is_json());
        assert!(CAPTURED.with(|captured| captured.borrow().is_none()));
        reset();
    }

    #[test]
    fn emitting_twice_is_prevented_by_the_unemitted_flush_guard() {
        reset();
        set_format(&OutputFormat::Json);
        emit("demo", Sample { name: "x".into() }, |s| s.name.clone());
        // The command already answered, so the safety net must stay silent.
        assert!(EMITTED.with(|emitted| *emitted.borrow()));
        flush_unemitted("demo");
        reset();
    }
}
