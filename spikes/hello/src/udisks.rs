use std::collections::HashMap;
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

type Interfaces = HashMap<String, HashMap<String, OwnedValue>>;

const DRIVE_IFACE: &str = "org.freedesktop.UDisks2.Drive";

pub fn list_drives() -> zbus::Result<Vec<String>> {
    let conn = Connection::system()?;
    let reply = conn.call_method(
        Some("org.freedesktop.UDisks2"),
        "/org/freedesktop/UDisks2",
        Some("org.freedesktop.DBus.ObjectManager"),
        "GetManagedObjects",
        &(),
    )?;
    let objects: HashMap<OwnedObjectPath, Interfaces> = reply.body().deserialize()?;
    let mut lines: Vec<String> = objects
        .values()
        .filter_map(|ifaces| ifaces.get(DRIVE_IFACE))
        .map(|props| {
            let text = |k: &str| -> String {
                props
                    .get(k)
                    .and_then(|v| v.try_clone().ok())
                    .and_then(|v| String::try_from(v).ok())
                    .unwrap_or_default()
            };
            let size = props
                .get("Size")
                .and_then(|v| v.try_clone().ok())
                .and_then(|v| u64::try_from(v).ok())
                .unwrap_or(0);
            let removable = props
                .get("Removable")
                .and_then(|v| v.try_clone().ok())
                .and_then(|v| bool::try_from(v).ok())
                .unwrap_or(false);
            format!(
                "{} {} - {} GB{}",
                text("Vendor"),
                text("Model"),
                size / 1_000_000_000,
                if removable { " [removable]" } else { "" }
            )
        })
        .collect();
    lines.sort();
    Ok(lines)
}
