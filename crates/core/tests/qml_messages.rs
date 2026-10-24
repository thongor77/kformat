//! Every message the QML (crates/app/qml) asks the controller for must exist in the
//! catalog, and be given exactly as many arguments as its text has placeholders.
//! Lives here rather than in the app crate because the app's test binaries cannot link
//! the generated Qt glue.
use std::path::Path;

use kformat_core::messages::Msg;

/// `(file, key, arguments given)` for each `….tr("Key", …)` / `tr1` / `tr2` / `tr3` call.
fn qml_message_calls(source: &str, file: &str) -> Vec<(String, String, usize)> {
    let mut calls = vec![];
    let mut rest = source;
    while let Some(at) = rest.find(".tr") {
        let after = &rest[at + 3..];
        rest = after;
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        let Some(args) = after[digits.len()..].strip_prefix('(') else {
            continue;
        };
        let args = args.trim_start();
        let Some(key_and_more) = args.strip_prefix('"') else {
            continue;
        };
        let Some(end) = key_and_more.find('"') else {
            continue;
        };
        let given = if digits.is_empty() {
            0
        } else {
            digits.parse().unwrap_or(0)
        };
        calls.push((file.to_owned(), key_and_more[..end].to_owned(), given));
    }
    calls
}

#[test]
fn every_qml_message_exists_and_gets_the_right_number_of_arguments() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/qml");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("qml directory") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_none_or(|ext| ext != "qml") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(&path).expect("readable qml file");
        for (file, key, given) in qml_message_calls(&source, &name) {
            let msg = Msg::from_key(&key)
                .unwrap_or_else(|| panic!("{file}: unknown message key {key:?}"));
            assert_eq!(
                msg.arg_count(),
                given,
                "{file}: {key} expects {} argument(s) but the QML passes {given}",
                msg.arg_count()
            );
            checked += 1;
        }
    }
    assert!(
        checked > 30,
        "only {checked} QML message calls found: is the scan broken?"
    );
}

#[test]
fn the_scanner_reads_keys_and_argument_counts() {
    let source = r#"text: controller.tr("A") + page.controller.tr2(
        "B", x, y) + c.tr1("C", z) + other.tree("D")"#;
    let calls = qml_message_calls(source, "t.qml");
    let found: Vec<_> = calls.iter().map(|(_, k, n)| (k.as_str(), *n)).collect();
    assert_eq!(found, vec![("A", 0), ("B", 2), ("C", 1)]);
}
