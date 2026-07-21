//! QEMU host probing and argument construction.

use core::net::SocketAddr;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;

use mikrotik_common::error_with_label;
use mikrotik_types::target::DeviceTarget;
use xshell::Shell;
use xshell::cmd;

use crate::DEFAULT_PASSWORD;
use crate::DEFAULT_USERNAME;
use crate::RuntimeSocketDir;
use crate::catalog::ChrArch;
use crate::catalog::RouterOsVersion;
use crate::catalog::guest_arch;
use crate::error::Error;
use crate::error::Result;

/// Owned running QEMU VM device.
#[derive(Debug)]
pub(crate) struct QemuVm {
    /// Router name used for diagnostics.
    name: String,
    /// Host socket forwarded to the `RouterOS` API service.
    api_socket: SocketAddr,
    /// Client target for this VM.
    target: DeviceTarget,
    /// Per-run artifact directory.
    run_dir: PathBuf,
    /// Runtime socket directory removed after this router is dropped.
    socket_dir_guard: Option<RuntimeSocketDir>,
    /// Owned QEMU child process.
    child: Option<Child>,
}

impl QemuVm {
    /// Build a running QEMU VM wrapper.
    pub(crate) fn new(name: String, api_port: u16, run_dir: PathBuf, child: Child) -> Result<Self> {
        Ok(Self {
            name,
            api_socket: SocketAddr::from(([127, 0, 0, 1], api_port)),
            target: target_for_port(api_port)?,
            run_dir,
            socket_dir_guard: None,
            child: Some(child),
        })
    }

    /// Attach a runtime socket directory guard to this VM.
    pub(crate) fn set_socket_dir_guard(&mut self, socket_dir_guard: RuntimeSocketDir) {
        self.socket_dir_guard = Some(socket_dir_guard);
    }

    /// Return the forwarded API socket.
    pub(crate) const fn api_socket(&self) -> SocketAddr {
        self.api_socket
    }

    /// Return the client target.
    pub(crate) fn target(&self) -> &DeviceTarget {
        &self.target
    }

    /// Return the per-run artifact directory.
    pub(crate) fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    /// Stop and reap the QEMU process.
    pub(crate) fn shutdown(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };

        if let Err(error) = child.kill() {
            error_with_label!(self.name, "Failed to stop: {error}");
        }
        if let Err(error) = child.wait() {
            error_with_label!(self.name, "Failed to reap: {error}");
        }
    }
}

impl Drop for QemuVm {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Build a crawler target for a forwarded localhost API port.
pub(crate) fn target_for_port(api_port: u16) -> Result<DeviceTarget> {
    DeviceTarget::new(
        format!("127.0.0.1:{api_port}"),
        DEFAULT_USERNAME,
        Some(DEFAULT_PASSWORD.to_owned()),
    )
    .map_err(|error| Error::Tool(format!("build target for API localhost:{api_port}: {error}")))
}

/// Runtime execution profile selected for one router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeTarget {
    /// CHR architecture matching the host.
    host_arch: ChrArch,
    /// CHR architecture selected for the guest image.
    pub(crate) guest_arch: ChrArch,
    /// QEMU acceleration mode selected for this host.
    accelerator: Accelerator,
}

impl RuntimeTarget {
    /// Detect guest architecture and acceleration mode for one `RouterOS` version.
    pub(crate) fn detect(
        sh: &Shell,
        host_arch: ChrArch,
        version: RouterOsVersion,
        allow_software: bool,
    ) -> Result<Self> {
        let guest_arch = guest_arch(host_arch, version)?;
        let accelerator = Accelerator::detect(sh, host_arch, guest_arch, allow_software)?;
        Ok(Self {
            host_arch,
            guest_arch,
            accelerator,
        })
    }

    /// Return the host architecture used to choose this target.
    pub(crate) const fn host_arch(self) -> ChrArch {
        self.host_arch
    }

    /// Return the selected QEMU acceleration mode name.
    pub(crate) const fn accelerator_name(self) -> &'static str {
        self.accelerator.name()
    }
}

/// QEMU acceleration mode selected for this host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Accelerator {
    /// macOS Hypervisor.framework acceleration.
    Hvf,
    /// Linux KVM acceleration.
    Kvm,
    /// QEMU TCG software emulation.
    Software,
}

impl Accelerator {
    /// Return a stable diagnostic name for this accelerator.
    const fn name(self) -> &'static str {
        match self {
            Self::Hvf => "hvf",
            Self::Kvm => "kvm",
            Self::Software => "tcg",
        }
    }

    /// Select a host acceleration mode or fail if software fallback is disallowed.
    fn detect(sh: &Shell, host_arch: ChrArch, guest_arch: ChrArch, allow_software: bool) -> Result<Self> {
        if host_arch != guest_arch {
            return Ok(Self::Software);
        }
        if cfg!(target_os = "macos") && guest_arch == ChrArch::Aarch64 {
            if allow_software {
                return Ok(Self::Software);
            }
            return Err(Error::Tool(
                "aarch64 CHR does not boot reliably with macOS HVF; enable software emulation to use TCG".to_owned(),
            ));
        }
        if cfg!(target_os = "macos") && host_hvf_available(sh) {
            return Ok(Self::Hvf);
        }
        if cfg!(target_os = "linux") && host_kvm_available() {
            return Ok(Self::Kvm);
        }
        if allow_software {
            Ok(Self::Software)
        } else {
            Err(Error::Tool(
                "hardware acceleration is unavailable; enable software emulation to use TCG".to_owned(),
            ))
        }
    }
}

/// Return whether Linux KVM is present and accessible to this process.
fn host_kvm_available() -> bool {
    OpenOptions::new().read(true).write(true).open("/dev/kvm").is_ok()
}

/// Return whether macOS Hypervisor.framework is available.
fn host_hvf_available(sh: &Shell) -> bool {
    sh.cmd("sysctl")
        .args(["-n", "kern.hv_support"])
        .read()
        .is_ok_and(|value| value.trim() == "1")
}

/// Find a usable QEMU system binary for the selected CHR architecture in `PATH`.
pub(crate) fn qemu_system_binary(sh: &Shell, arch: ChrArch) -> Result<String> {
    let candidates: &[&str] = match arch {
        ChrArch::X86_64 => &["qemu-system-x86_64", "qemu-system-amd64"],
        ChrArch::Aarch64 => &["qemu-system-aarch64"],
    };
    for candidate in candidates {
        if tool_exists(sh, candidate) {
            return Ok((*candidate).to_owned());
        }
    }
    Err(Error::Tool(format!(
        "missing QEMU system binary for {arch:?} in PATH: tried {}",
        candidates.join(", ")
    )))
}

/// Require a host tool to be available in `PATH`.
pub(crate) fn ensure_tool(sh: &Shell, tool: &str) -> Result<()> {
    if tool_exists(sh, tool) {
        Ok(())
    } else {
        Err(Error::Tool(format!("missing `{tool}` in PATH")))
    }
}

/// Return whether a host tool can be executed with `--version`.
fn tool_exists(sh: &Shell, tool: &str) -> bool {
    sh.cmd(tool).arg("--version").ignore_status().run().is_ok()
}

/// Create a qcow2 overlay backed by the cached CHR raw image.
pub(crate) fn create_overlay(sh: &Shell, base_image: &Path, overlay: &Path) -> Result<()> {
    cmd!(sh, "qemu-img create -f qcow2 -F raw -b {base_image} {overlay}")
        .run()
        .map_err(|error| Error::Tool(format!("create qcow2 overlay {}: {error}", overlay.display())))
}

/// Append QEMU acceleration arguments.
pub(crate) fn append_accelerator_args(args: &mut Vec<String>, target: RuntimeTarget) {
    match target.accelerator {
        Accelerator::Hvf => args.extend(["-accel".to_owned(), "hvf".to_owned()]),
        Accelerator::Kvm => args.extend(["-accel".to_owned(), "kvm".to_owned()]),
        Accelerator::Software => {
            let value = if target.host_arch == target.guest_arch {
                "tcg"
            } else {
                "tcg,tb-size=256"
            };
            args.extend(["-accel".to_owned(), value.to_owned()]);
        }
    }
}

/// Append architecture-specific QEMU machine and disk arguments.
pub(crate) fn append_disk_args(
    args: &mut Vec<String>,
    target: RuntimeTarget,
    overlay: &Path,
    router_name: &str,
    run_dir: &Path,
    sh: &Shell,
) -> Result<()> {
    match target.guest_arch {
        ChrArch::X86_64 => {
            args.extend([
                "-M".to_owned(),
                "q35".to_owned(),
                "-drive".to_owned(),
                format!("file={},if=virtio,format=qcow2", overlay.display()),
            ]);
        }
        ChrArch::Aarch64 => {
            let firmware = aarch64_firmware_paths()?;
            let vars = run_dir.join(format!("{router_name}.vars.fd"));
            sh.copy_file(&firmware.vars, &vars).map_err(|error| {
                Error::Tool(format!(
                    "copy aarch64 firmware vars from {} to {}: {error}",
                    firmware.vars.display(),
                    vars.display()
                ))
            })?;
            args.extend([
                "-M".to_owned(),
                "virt,acpi=on".to_owned(),
                "-cpu".to_owned(),
                match target.accelerator {
                    Accelerator::Hvf => "host".to_owned(),
                    Accelerator::Kvm | Accelerator::Software => "cortex-a710".to_owned(),
                },
                "-drive".to_owned(),
                format!(
                    "if=pflash,format=raw,readonly=on,unit=0,file={}",
                    firmware.code.display()
                ),
                "-drive".to_owned(),
                format!("if=pflash,format=raw,unit=1,file={}", vars.display()),
                "-drive".to_owned(),
                format!("file={},format=qcow2,if=none,id=drive0", overlay.display()),
                "-device".to_owned(),
                "virtio-blk-pci,drive=drive0,addr=0x1".to_owned(),
            ]);
        }
    }
    Ok(())
}

/// aarch64 UEFI firmware files required by QEMU.
struct Aarch64Firmware {
    /// Read-only code pflash.
    code: PathBuf,
    /// Writable vars pflash template.
    vars: PathBuf,
}

/// Locate aarch64 EDK2 firmware paths for common QEMU installations.
fn aarch64_firmware_paths() -> Result<Aarch64Firmware> {
    for (code, vars) in [
        (
            "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
            "/opt/homebrew/share/qemu/edk2-arm-vars.fd",
        ),
        (
            "/usr/local/share/qemu/edk2-aarch64-code.fd",
            "/usr/local/share/qemu/edk2-arm-vars.fd",
        ),
        ("/usr/share/AAVMF/AAVMF_CODE.fd", "/usr/share/AAVMF/AAVMF_VARS.fd"),
    ] {
        let code = PathBuf::from(code);
        let vars = PathBuf::from(vars);
        if code.exists() && vars.exists() {
            return Ok(Aarch64Firmware { code, vars });
        }
    }
    Err(Error::Tool("missing aarch64 EDK2 firmware files for QEMU".to_owned()))
}
