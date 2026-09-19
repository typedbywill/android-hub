use android_hub_core::{adb, select_device, session};
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use std::{
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Parser)]
#[command(
    name = "android-hub",
    about = "Use Android hardware as Linux input devices over USB debugging"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Doctor,
    Devices,
    Setup {
        #[command(subcommand)]
        command: Setup,
    },
    Cameras(DeviceArgs),
    Start(StartArgs),
}
#[derive(Subcommand)]
enum Setup {
    Android {
        #[arg(long)]
        device: Option<String>,
        #[arg(
            long,
            default_value = "android/app/build/outputs/apk/debug/app-debug.apk"
        )]
        apk: PathBuf,
    },
}
#[derive(Args)]
struct DeviceArgs {
    #[arg(long)]
    device: Option<String>,
}
#[derive(Args)]
struct StartArgs {
    #[arg(long)]
    device: Option<String>,
    #[arg(long)]
    mic: bool,
    #[arg(long)]
    webcam: bool,
    #[arg(long, default_value = "/dev/video10")]
    v4l2_sink: String,
    #[arg(long, default_value = "front")]
    camera: String,
}
#[derive(Debug, Serialize)]
struct Check {
    name: &'static str,
    available: bool,
    hint: &'static str,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor => doctor(cli.json),
        Commands::Devices => devices(cli.json),
        Commands::Setup {
            command: Setup::Android { device, apk },
        } => setup(device, apk),
        Commands::Cameras(args) => cameras(args.device, cli.json),
        Commands::Start(args) => start(args),
    }
}
fn emit<T: Serialize + std::fmt::Debug>(value: &T, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(value).unwrap());
    } else {
        println!("{value:#?}");
    }
}
fn doctor(json: bool) -> Result<()> {
    let checks = vec![
        Check {
            name: "adb",
            available: which::which("adb").is_ok(),
            hint: "Install Android platform-tools.",
        },
        Check {
            name: "scrcpy",
            available: which::which("scrcpy").is_ok(),
            hint: "Install scrcpy for Android camera forwarding.",
        },
        Check {
            name: "pactl",
            available: which::which("pactl").is_ok(),
            hint: "Install PipeWire with pipewire-pulse.",
        },
        Check {
            name: "pw-cat",
            available: which::which("pw-cat").is_ok(),
            hint: "Install PipeWire tools.",
        },
        Check {
            name: "v4l2loopback",
            available: std::path::Path::new("/dev/video10").exists(),
            hint: "Provision a v4l2loopback device, then use --v4l2-sink.",
        },
    ];
    emit(&checks, json);
    if checks.iter().all(|c| c.available) {
        Ok(())
    } else {
        anyhow::bail!("One or more required dependencies are unavailable")
    }
}
fn devices(json: bool) -> Result<()> {
    emit(&adb::devices()?, json);
    Ok(())
}
fn selected(serial: Option<String>) -> Result<android_hub_core::Device> {
    select_device(&adb::devices()?, serial.as_deref())
}
fn setup(serial: Option<String>, apk: PathBuf) -> Result<()> {
    let device = selected(serial)?;
    if !apk.is_file() {
        anyhow::bail!(
            "APK not found at {}. Build it with ./gradlew :app:assembleDebug from android/.",
            apk.display()
        );
    }
    adb::install(&device.serial, apk.to_str().unwrap())?;
    println!(
        "Installed Android Hub on {}. Open it and grant microphone permission.",
        device.serial
    );
    Ok(())
}
fn cameras(serial: Option<String>, json: bool) -> Result<()> {
    let device = selected(serial)?;
    if device.android_version.unwrap_or_default() < 31 {
        anyhow::bail!("Camera forwarding needs Android 12 (API 31) or newer");
    }
    let output = Command::new("scrcpy")
        .args(["--serial", &device.serial, "--list-cameras"])
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "scrcpy could not list cameras: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    if json {
        println!("{}", serde_json::json!({"device":device.serial,"raw":raw}));
    } else {
        println!("Camera capabilities for {}:\n{}", device.serial, raw);
    }
    Ok(())
}
fn start(args: StartArgs) -> Result<()> {
    if !args.mic && !args.webcam {
        anyhow::bail!("Choose at least one module: --mic and/or --webcam");
    }
    let device = selected(args.device)?;
    if args.webcam && device.android_version.unwrap_or_default() < 31 {
        anyhow::bail!("Webcam requires Android 12 or newer");
    }
    if args.webcam && !std::path::Path::new(&args.v4l2_sink).exists() {
        anyhow::bail!(
            "{} does not exist. Provision v4l2loopback first.",
            args.v4l2_sink
        );
    }
    let mut audio = if args.mic {
        Some(session::AudioBridge::start(&device.serial)?)
    } else {
        None
    };
    let mut webcam = if args.webcam {
        Some(session::WebcamBridge::start(
            &device.serial,
            &args.v4l2_sink,
            &args.camera,
        )?)
    } else {
        None
    };
    println!(
        "Android Hub session active for {}. Press Ctrl+C to stop.",
        device.serial
    );
    if audio.is_some() {
        println!("Microphone: Android Hub Microphone.monitor");
    }
    if webcam.is_some() {
        println!("Webcam: {}", args.v4l2_sink);
    }
    let running = Arc::new(AtomicBool::new(true));
    let stop = running.clone();
    ctrlc::set_handler(move || stop.store(false, Ordering::SeqCst))?;
    while running.load(Ordering::SeqCst) {
        if let Some(audio) = audio.as_mut() {
            audio.check_health()?;
        }
        if let Some(webcam) = webcam.as_mut() {
            webcam.check_health()?;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    drop(webcam);
    drop(audio);
    println!("Android Hub session stopped.");
    Ok(())
}
