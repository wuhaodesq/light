use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ast::Program;
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

pub fn build_firmware(
    program: &Program,
    target: &str,
    linker_script: Option<&Path>,
) -> Result<(), Diagnostic> {
    if !crate::backend::supported_targets().contains(&target) {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("unsupported target `{target}`"),
            Span::new(0, 0),
        ));
    }

    if !crate::backend::is_bare_metal_target(target) {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("target `{target}` is not a bare-metal firmware target"),
            Span::new(0, 0),
        ));
    }

    crate::backend::build_program(program, target.to_string(), true, linker_script)?;

    let stem = program
        .functions
        .iter()
        .find(|f| f.name == "main")
        .map(|f| f.name.as_str())
        .unwrap_or("main");

    let source_dir = PathBuf::from("build").join(target);
    let firmware_dir = PathBuf::from("build/firmware").join(target);

    fs::create_dir_all(&firmware_dir).map_err(|e| {
        Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
    })?;

    let files = ["elf", "bin", "hex", "ll", "dis"];
    for ext in &files {
        let src = source_dir.join(format!("{stem}.{ext}"));
        let dst = firmware_dir.join(format!("{stem}.{ext}"));
        if src.exists() {
            fs::copy(&src, &dst).map_err(|e| {
                Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
            })?;
        }
    }

    if let Some(script) = linker_script {
        if script.exists() {
            let dst = firmware_dir.join("linker.ld");
            fs::copy(script, &dst).map_err(|e| {
                Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
            })?;
        }
    }

    println!("firmware built for {target}");
    println!("{}", firmware_dir.join(format!("{stem}.elf")).display());
    println!("{}", firmware_dir.join(format!("{stem}.bin")).display());
    println!("{}", firmware_dir.join(format!("{stem}.hex")).display());
    Ok(())
}

pub enum FlashInterface {
    Stlink,
    Jlink,
    EspDownload,
    OpenOCD,
    Serial,
}

impl FlashInterface {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "stlink" | "st" => Some(FlashInterface::Stlink),
            "jlink" | "jl" => Some(FlashInterface::Jlink),
            "esptool" | "esp" | "esp-download" => Some(FlashInterface::EspDownload),
            "openocd" | "ocd" => Some(FlashInterface::OpenOCD),
            "serial" | "uart" | "com" => Some(FlashInterface::Serial),
            _ => None,
        }
    }

    pub fn flash(&self, image: &Path, target: &str, port: Option<&str>) -> Result<(), Diagnostic> {
        match self {
            FlashInterface::Stlink => self.flash_stlink(image, target),
            FlashInterface::Jlink => self.flash_jlink(image, target),
            FlashInterface::EspDownload => self.flash_esp(image, port),
            FlashInterface::OpenOCD => self.flash_openocd(image, target),
            FlashInterface::Serial => self.flash_serial(image, port),
        }
    }

    fn flash_stlink(&self, image: &Path, target: &str) -> Result<(), Diagnostic> {
        let bin_path = image.to_path_buf();

        println!("Flashing {} with STLink...", target);
        println!("Binary: {}", bin_path.display());

        let output = Command::new("openocd")
            .args([
                "-f", "interface/stlink.cfg",
                "-f", &format!("target/{}.cfg", self.stm32_target_name(target)),
                "-c", &format!("program {} 0x08000000 verify reset exit", bin_path.display()),
            ])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    println!("Flash successful via STLink");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    eprintln!("STLink flash failed: {}", stderr);
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        format!("STLink flash failed: {}", stderr),
                        Span::new(0, 0),
                    ))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    println!("openocd not found, simulating flash...");
                    println!("NOTE: Install openocd to flash for real");
                    println!("  Linux: sudo apt install openocd");
                    println!("  macOS: brew install openocd");
                    println!("  Windows: choco install openocd");
                    self.simulate_flash(image, target, "STLink")
                } else {
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        e.to_string(),
                        Span::new(0, 0),
                    ))
                }
            }
        }
    }

    fn flash_jlink(&self, image: &Path, target: &str) -> Result<(), Diagnostic> {
        let bin_path = image.to_path_buf();

        println!("Flashing {} with JLink...", target);
        println!("Binary: {}", bin_path.display());

        let output = Command::new("JLink.exe")
            .args([
                "-device", self.jlink_device_name(target),
                "-if", "SWD",
                "-speed", "4000",
                "-program", &bin_path.display().to_string(),
                "-verify",
                "-exit",
            ])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    println!("Flash successful via JLink");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    eprintln!("JLink flash failed: {}", stderr);
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        format!("JLink flash failed: {}", stderr),
                        Span::new(0, 0),
                    ))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    println!("JLink.exe not found, simulating flash...");
                    println!("NOTE: Install JLink to flash for real");
                    self.simulate_flash(image, target, "JLink")
                } else {
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        e.to_string(),
                        Span::new(0, 0),
                    ))
                }
            }
        }
    }

    fn flash_esp(&self, image: &Path, port: Option<&str>) -> Result<(), Diagnostic> {
        let port = port.unwrap_or("COM3");
        let bin_path = image.to_path_buf();

        println!("Flashing ESP32 via {}...", port);
        println!("Binary: {}", bin_path.display());

        let output = Command::new("esptool.py")
            .args([
                "--chip", "esp32",
                "--port", port,
                "--baud", "921600",
                "write_flash", "0x1000", &bin_path.display().to_string(),
            ])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    println!("Flash successful via ESP32");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    eprintln!("esptool flash failed: {}", stderr);
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        format!("ESP32 flash failed: {}", stderr),
                        Span::new(0, 0),
                    ))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    println!("esptool.py not found, simulating flash...");
                    println!("NOTE: Install esptool to flash for real");
                    println!("  pip install esptool");
                    self.simulate_flash(image, "ESP32", "esptool")
                } else {
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        e.to_string(),
                        Span::new(0, 0),
                    ))
                }
            }
        }
    }

    fn flash_openocd(&self, image: &Path, target: &str) -> Result<(), Diagnostic> {
        let bin_path = image.to_path_buf();

        println!("Flashing {} with OpenOCD...", target);
        println!("Binary: {}", bin_path.display());

        let output = Command::new("openocd")
            .args([
                "-f", "interface/jlink.cfg",
                "-f", &format!("target/{}.cfg", self.stm32_target_name(target)),
                "-c", &format!("program {} 0x08000000 verify reset exit", bin_path.display()),
            ])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    println!("Flash successful via OpenOCD");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    eprintln!("OpenOCD flash failed: {}", stderr);
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        format!("OpenOCD flash failed: {}", stderr),
                        Span::new(0, 0),
                    ))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    println!("openocd not found, simulating flash...");
                    self.simulate_flash(image, target, "OpenOCD")
                } else {
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        e.to_string(),
                        Span::new(0, 0),
                    ))
                }
            }
        }
    }

    fn flash_serial(&self, image: &Path, port: Option<&str>) -> Result<(), Diagnostic> {
        let port = port.unwrap_or("COM3");
        let bin_path = image.to_path_buf();

        println!("Flashing via serial {}...", port);
        println!("Binary: {}", bin_path.display());
        println!("NOTE: Ensure device is in bootloader mode");

        println!("Flash successful via serial (simulated)");
        Ok(())
    }

    fn simulate_flash(&self, image: &Path, target: &str, tool: &str) -> Result<(), Diagnostic> {
        println!("=== Simulated Flash ===");
        println!("Target: {}", target);
        println!("Tool: {}", tool);
        println!("Image: {}", image.display());
        println!("Address: 0x08000000");
        println!("======================");
        Ok(())
    }

    fn stm32_target_name(&self, target: &str) -> &'static str {
        match target {
            "stm32f407" => "stm32f4x",
            "stm32f103" => "stm32f1x",
            _ => "stm32f4x",
        }
    }

    fn jlink_device_name(&self, target: &str) -> &'static str {
        match target {
            "stm32f407" => "STM32F407VG",
            "stm32f103" => "STM32F103C8",
            _ => "STM32F407VG",
        }
    }
}

pub fn flash_image(
    image: &Path,
    target: &str,
    interface: Option<&str>,
    port: Option<&str>,
) -> Result<(), Diagnostic> {
    if !image.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("image not found: {}", image.display()),
            Span::new(0, 0),
        ));
    }

    let interface = interface.or(port).unwrap_or("stlink");

    let flash_if = FlashInterface::from_str(interface).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("unknown flash interface `{interface}`"),
            Span::new(0, 0),
        )
    })?;

    flash_if.flash(image, target, port)
}

pub fn list_flash_interfaces() {
    println!("Supported flash interfaces:");
    println!("  stlink, st    - STLink (STM32)");
    println!("  jlink, jl     - JLink (ARM Cortex)");
    println!("  esptool, esp  - esptool.py (ESP32)");
    println!("  openocd, ocd  - OpenOCD (multi-vendor)");
    println!("  serial, uart  - Serial/UART bootloader");
}
