//! Shared domain types for Android Hub clients.
pub mod adb;
pub mod protocol;
pub mod session;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
    pub android_version: Option<u32>,
}

impl Device {
    pub fn ready(&self) -> bool {
        self.state == "device"
    }
}

pub fn select_device(devices: &[Device], requested: Option<&str>) -> anyhow::Result<Device> {
    if let Some(serial) = requested {
        return devices
            .iter()
            .find(|d| d.serial == serial)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Android device '{serial}' was not found"));
    }
    let mut ready: Vec<_> = devices.iter().filter(|d| d.ready()).cloned().collect();
    match ready.len() {
        0 => anyhow::bail!(
            "No authorized Android device found. Enable USB debugging and accept the RSA prompt."
        ),
        1 => Ok(ready.remove(0)),
        _ => anyhow::bail!("More than one Android device is connected. Pass --device SERIAL."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn device(serial: &str) -> Device {
        Device {
            serial: serial.into(),
            state: "device".into(),
            model: None,
            android_version: None,
        }
    }
    #[test]
    fn picks_the_only_ready_device() {
        assert_eq!(select_device(&[device("a")], None).unwrap().serial, "a");
    }
    #[test]
    fn requires_serial_for_multiple_devices() {
        assert!(select_device(&[device("a"), device("b")], None).is_err());
    }
    #[test]
    fn requested_device_is_selected() {
        assert_eq!(
            select_device(&[device("a"), device("b")], Some("b"))
                .unwrap()
                .serial,
            "b"
        );
    }
}
