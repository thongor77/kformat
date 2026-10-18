mod disk_controller;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

const MAIN_QML: &str = "qrc:/qt/qml/org/kformat/app/qml/Main.qml";

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(MAIN_QML));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
