//! Plugin manifest consistency test (task 3.4).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]

use openidea_plugin_kiro::commands::all_commands;
use openidea_plugin_kiro::registry::static_commands;
use toml::Value;

#[test]
fn manifest_contains_valid_fields_and_matches_registry_static_list() {
    let manifest_str = include_str!("../plugin.toml");
    let parsed: Value = toml::from_str(manifest_str).expect("plugin.toml must be valid TOML");

    assert_eq!(
        parsed.get("id").and_then(Value::as_str),
        Some("kiro"),
        "manifest id must be 'kiro'"
    );
    assert_eq!(
        parsed.get("name").and_then(Value::as_str),
        Some("Kiro"),
        "manifest name must be 'Kiro'"
    );
    assert_eq!(
        parsed.get("version").and_then(Value::as_str),
        Some("1.0.0"),
        "manifest version must be '1.0.0'"
    );
    assert_eq!(
        parsed.get("openidea-api").and_then(Value::as_str),
        Some("0.1"),
        "openidea-api must be '0.1'"
    );
    assert_eq!(
        parsed.get("wasm").and_then(Value::as_str),
        Some("dist/plugin.wasm"),
        "wasm must point to 'dist/plugin.wasm'"
    );

    let permissions = parsed
        .get("permissions")
        .and_then(Value::as_array)
        .expect("permissions array required");
    let perm_strings: Vec<&str> = permissions.iter().filter_map(Value::as_str).collect();
    assert_eq!(
        perm_strings,
        vec!["project-read", "editor-read", "editor-write"],
        "permissions must match the required three permissions"
    );

    let contributes = parsed
        .get("contributes")
        .and_then(Value::as_table)
        .expect("contributes table required");
    let commands = contributes
        .get("commands")
        .and_then(Value::as_array)
        .expect("contributes.commands array required");

    let expected_decls = static_commands(&all_commands());
    assert_eq!(
        commands.len(),
        expected_decls.len(),
        "number of static commands in manifest must equal registry static list"
    );

    for (idx, (cmd_val, expected)) in commands.iter().zip(expected_decls.iter()).enumerate() {
        let cmd_table = cmd_val
            .as_table()
            .unwrap_or_else(|| panic!("command entry {idx} must be a table"));
        let id = cmd_table
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("command {idx} missing id"));
        let title = cmd_table
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("command {idx} missing title"));
        let category = cmd_table.get("category").and_then(Value::as_str);

        assert_eq!(
            id, expected.id,
            "command {idx} id mismatch: expected '{}', got '{}'",
            expected.id, id
        );
        assert_eq!(
            title, expected.title,
            "command {idx} title mismatch: expected '{}', got '{}'",
            expected.title, title
        );
        assert_eq!(
            category,
            Some("Kiro"),
            "command {idx} category mismatch: expected Some(\"Kiro\"), got '{:?}'",
            category
        );
    }
}
