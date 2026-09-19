use crate::{adb, protocol};
use anyhow::{Context, Result};
use std::{
    io::Write,
    net::TcpStream,
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

pub const MIC_DEVICE_PORT: u16 = 61_394;
pub const MIC_HOST_PORT: u16 = 61_394;

fn cleanup_partial_audio_bridge(null_sink: u32, writer: &mut Child, serial: &str) {
    let _ = writer.kill();
    let _ = Command::new("pactl")
        .args(["unload-module", &null_sink.to_string()])
        .status();
    adb::remove_forward(serial, MIC_HOST_PORT);
}

pub struct AudioBridge {
    null_sink: u32,
    writer: Child,
    receiver: Option<thread::JoinHandle<Result<()>>>,
    serial: String,
}
impl AudioBridge {
    pub fn start(serial: &str) -> Result<Self> {
        let output = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                "sink_name=android_hub_mic",
                "sink_properties=device.description=Android_Hub_Microphone",
            ])
            .output()
            .context("Could not run pactl; PipeWire with pipewire-pulse is required")?;
        if !output.status.success() {
            anyhow::bail!(
                "Could not create PipeWire sink: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let null_sink = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .context("Unexpected pactl module id")?;
        let mut writer = Command::new("pw-cat")
            .args([
                "--playback",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                "1",
                "--target",
                "android_hub_mic",
                "-",
            ])
            .stdin(Stdio::piped())
            .spawn()
            .context("Could not run pw-cat")?;
        if let Err(error) = adb::forward(serial, MIC_HOST_PORT, MIC_DEVICE_PORT) {
            cleanup_partial_audio_bridge(null_sink, &mut writer, serial);
            return Err(error);
        }
        let mut stream = match (0..20)
            .find_map(|_| match TcpStream::connect(("127.0.0.1", MIC_HOST_PORT)) {
                Ok(stream) => Some(Ok(stream)),
                Err(_) => {
                    thread::sleep(Duration::from_millis(150));
                    None
                }
            })
            .unwrap_or_else(|| Err(anyhow::anyhow!("Android microphone service is not ready. Open Android Hub and grant microphone permission.")))
        {
            Ok(stream) => stream,
            Err(error) => {
                cleanup_partial_audio_bridge(null_sink, &mut writer, serial);
                return Err(error);
            }
        };
        let format = match protocol::read_header(&mut stream) {
            Ok(format) => format,
            Err(error) => {
                cleanup_partial_audio_bridge(null_sink, &mut writer, serial);
                return Err(error.into());
            }
        };
        if format != protocol::AudioFormat::PCM_48K_MONO {
            cleanup_partial_audio_bridge(null_sink, &mut writer, serial);
            anyhow::bail!("Android sent an unsupported audio format");
        }
        let mut stdin = writer
            .stdin
            .take()
            .context("pw-cat did not provide stdin")?;
        let receiver = thread::spawn(move || -> Result<()> {
            loop {
                let frame =
                    protocol::read_frame(&mut stream).context("Android audio connection ended")?;
                stdin
                    .write_all(&frame)
                    .context("PipeWire audio stream ended")?;
            }
        });
        Ok(Self {
            null_sink,
            writer,
            receiver: Some(receiver),
            serial: serial.to_owned(),
        })
    }
    /// Returns an error immediately once the USB transport or PipeWire writer stops.
    pub fn check_health(&mut self) -> Result<()> {
        if self
            .receiver
            .as_ref()
            .is_some_and(|handle| handle.is_finished())
        {
            return self
                .receiver
                .take()
                .unwrap()
                .join()
                .map_err(|_| anyhow::anyhow!("Android audio receiver panicked"))?;
        }
        Ok(())
    }
}
impl Drop for AudioBridge {
    fn drop(&mut self) {
        let _ = self.writer.kill();
        let _ = Command::new("pactl")
            .args(["unload-module", &self.null_sink.to_string()])
            .status();
        adb::remove_forward(&self.serial, MIC_HOST_PORT);
    }
}

pub struct WebcamBridge {
    child: Child,
}
impl WebcamBridge {
    pub fn start(serial: &str, v4l2_sink: &str, camera: &str) -> Result<Self> {
        let child = Command::new("scrcpy")
            .args([
                "--serial",
                serial,
                "--video-source=camera",
                "--no-audio",
                "--no-video-playback",
                "--camera-facing",
                camera,
                "--camera-size=1280x720",
                "--camera-fps=30",
                "--v4l2-sink",
                v4l2_sink,
            ])
            .spawn()
            .context("Could not start scrcpy")?;
        Ok(Self { child })
    }
    pub fn check_health(&mut self) -> Result<()> {
        if let Some(status) = self.child.try_wait()? {
            anyhow::bail!("scrcpy webcam process stopped ({status})");
        }
        Ok(())
    }
}
impl Drop for WebcamBridge {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
