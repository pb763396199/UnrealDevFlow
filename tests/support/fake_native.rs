#![allow(dead_code)]

use serde_json::json;
use std::fs;
use std::path::Path;

/// Writes the same minimal execution shape that a fake native process would
/// leave behind.  The lifecycle tests use it to exercise status/compare
/// without launching an Editor in CI.
pub fn write_record(
    config_dir: &Path,
    id: &str,
    state: &str,
    digest: &str,
    project: &Path,
    engine: &Path,
    map: Option<&str>,
) {
    let root = config_dir.join("executions/run").join(id);
    fs::create_dir_all(&root).unwrap();
    let tool_exit = match state {
        "passed" => Some(0),
        "failed" => Some(1),
        _ => None,
    };
    let record = json!({
        "schemaVersion": 1,
        "executionId": id,
        "name": "fake-native",
        "profileRevision": 1,
        "profileDigest": digest,
        "scope": {"kind": "workspace", "value": "fixture"},
        "resolvedTarget": {
            "source": "workspace",
            "workspace": "fixture",
            "task": null,
            "project": project,
            "engine": engine,
            "host": null,
            "map": map,
            "mapPath": null,
            "mode": "editor",
            "rhi": "default"
        },
        "nativeExecutable": "fake-native.exe",
        "nativeArgv": ["--fixture"],
        "startedAt": "2026-09-05T00:00:00Z",
        "finishedAt": "2026-09-05T00:00:01Z",
        "state": state,
        "testResult": state,
        "exitResult": null,
        "businessResult": null,
        "toolExitCode": tool_exit,
        "ueExitCode": null,
        "pid": null,
        "artifacts": [],
        "diagnostics": []
    });
    fs::write(
        root.join("record.json"),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
}
