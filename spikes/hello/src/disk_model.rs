#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, summary)]
        type DiskModel = super::DiskModelRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);
    }
}

use core::pin::Pin;
use cxx_qt_lib::QString;

#[derive(Default)]
pub struct DiskModelRust {
    summary: QString,
}

impl qobject::DiskModel {
    pub fn refresh(self: Pin<&mut Self>) {
        let text = match crate::udisks::list_drives() {
            Ok(lines) => lines.join("\n"),
            Err(err) => format!("udisks error: {err}"),
        };
        println!("{text}");
        self.set_summary(QString::from(text.as_str()));
    }
}
