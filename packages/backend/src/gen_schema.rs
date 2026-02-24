#[allow(dead_code, unused_imports)]
mod cost;
#[allow(dead_code, unused_imports)]
mod providers;
#[allow(dead_code, unused_imports)]
mod server;
#[allow(dead_code, unused_imports)]
mod session;
mod types;

use schemars::schema_for;
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

fn extract_and_collect(
    name: &str,
    schema: Value,
    definitions: &mut Map<String, Value>,
) {
    // Collect definitions from this schema
    if let Some(Value::Object(defs)) = schema.get("definitions") {
        for (k, v) in defs {
            definitions.insert(k.clone(), v.clone());
        }
    }

    // Build the top-level entry: strip definitions and $schema, keep the rest
    let mut entry = schema;
    if let Value::Object(ref mut obj) = entry {
        obj.remove("definitions");
        obj.remove("$schema");
    }
    definitions.insert(name.to_string(), entry);
}

fn main() {
    let schema_dir = Path::new("packages/backend/schema");
    fs::create_dir_all(schema_dir).expect("Failed to create schema directory");

    let mut definitions = Map::new();

    let types: Vec<(&str, Value)> = vec![
        (
            "AgentSessionSummary",
            serde_json::to_value(schema_for!(types::AgentSessionSummary)).unwrap(),
        ),
        (
            "AgentSessionDetail",
            serde_json::to_value(schema_for!(types::AgentSessionDetail)).unwrap(),
        ),
        (
            "AgentMessage",
            serde_json::to_value(schema_for!(types::AgentMessage)).unwrap(),
        ),
        (
            "ServerEvent",
            serde_json::to_value(schema_for!(types::ServerEvent)).unwrap(),
        ),
        (
            "ClientEvent",
            serde_json::to_value(schema_for!(types::ClientEvent)).unwrap(),
        ),
        (
            "CumulativeUsage",
            serde_json::to_value(schema_for!(types::CumulativeUsage)).unwrap(),
        ),
        (
            "AgentStateType",
            serde_json::to_value(schema_for!(types::AgentStateType)).unwrap(),
        ),
        (
            "SearchScope",
            serde_json::to_value(schema_for!(types::SearchScope)).unwrap(),
        ),
        (
            "SearchMatch",
            serde_json::to_value(schema_for!(types::SearchMatch)).unwrap(),
        ),
        (
            "SessionSearchResult",
            serde_json::to_value(schema_for!(types::SessionSearchResult)).unwrap(),
        ),
        (
            "SearchResponse",
            serde_json::to_value(schema_for!(types::SearchResponse)).unwrap(),
        ),
        (
            "GitStatus",
            serde_json::to_value(schema_for!(types::GitStatus)).unwrap(),
        ),
    ];

    for (name, schema) in types {
        extract_and_collect(name, schema, &mut definitions);
    }

    let combined = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "AgentsDashboard",
        "definitions": definitions,
        "type": "object",
    });

    let path = schema_dir.join("all.json");
    let json = serde_json::to_string_pretty(&combined).expect("Failed to serialize schema");
    fs::write(&path, json).expect("Failed to write schema file");
    println!("Generated: {}", path.display());

    // Clean up old individual schema files
    let old_files = [
        "AgentSessionSummary.json",
        "AgentSessionDetail.json",
        "AgentMessage.json",
        "ServerEvent.json",
        "ClientEvent.json",
        "CumulativeUsage.json",
        "AgentStateType.json",
    ];
    for name in &old_files {
        let old_path = schema_dir.join(name);
        if old_path.exists() {
            fs::remove_file(&old_path).ok();
            println!("Removed old schema: {}", old_path.display());
        }
    }
}
