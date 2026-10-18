use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    let mut qml_files = vec![
        "qml/Main.qml",
        "qml/DiskPage.qml",
        "qml/OptionsPage.qml",
        "qml/ConfirmPage.qml",
        "qml/RunPage.qml",
    ];
    if std::env::var_os("CARGO_FEATURE_SMOKE").is_some() {
        qml_files.push("qml/SmokeTest.qml");
    }
    CxxQtBuilder::new_qml_module(QmlModule::new("org.kformat.app").qml_files(qml_files))
        .qt_module("Network")
        .files(["src/disk_controller.rs"])
        .build();
}
