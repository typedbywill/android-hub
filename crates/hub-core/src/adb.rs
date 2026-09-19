use crate::Device;
use anyhow::{Context, Result};
use std::process::Command;

pub fn command(serial: Option<&str>, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("adb");
    if let Some(serial) = serial {
        cmd.args(["-s", serial]);
    }
    let output = cmd
        .args(args)
        .output()
        .context("Could not execute adb; install Android platform-tools")?;
    if !output.status.success() {
        anyhow::bail!("adb {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn devices() -> Result<Vec<Device>> {
    let text = command(None, &["devices", "-l"])?;
    let mut devices = Vec::new();
    for line in text.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let serial = fields.next().unwrap_or_default().to_owned();
        let state = fields.next().unwrap_or_default().to_owned();
        let model = line
            .split_whitespace()
            .find_map(|part| part.strip_prefix("model:"))
            .map(str::to_owned);
        let android_version = if state == "device" {
            command(Some(&serial), &["shell", "getprop", "ro.build.version.sdk"])
                .ok()
                .and_then(|v| v.parse().ok())
        } else {
            None
        };
        devices.push(Device {
            serial,
            state,
            model,
            android_version,
        });
    }
    Ok(devices)
}

pub fn forward(serial: &str, host_port: u16, device_port: u16) -> Result<()> {
    command(
        Some(serial),
        &[
            "forward",
            &format!("tcp:{host_port}"),
            &format!("tcp:{device_port}"),
        ],
    )
    .map(|_| ())
}
pub fn remove_forward(serial: &str, host_port: u16) {
    let _ = command(
        Some(serial),
        &["forward", "--remove", &format!("tcp:{host_port}")],
    );
}
pub fn start_mic_service(serial: &str) -> Result<()> {
    command(
        Some(serial),
        &[
            "shell",
            "am",
            "start-foreground-service",
            "-n",
            "dev.androidhub/.MicStreamService",
        ],
    )
    .map(|_| ())
}
pub fn install(serial: &str, apk: &str) -> Result<()> {
    command(Some(serial), &["install", "-r", apk]).map(|_| ())
}
