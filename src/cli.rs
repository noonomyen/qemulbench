use clap::{Parser, Subcommand, ValueEnum};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Architecture {
    #[value(name = "x86_64", alias = "amd64", alias = "x86")]
    X86_64,
    #[value(name = "aarch64", alias = "arm64")]
    Aarch64,
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        }
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccelMode {
    #[value(name = "kvm")]
    Kvm,
    #[value(name = "tcg")]
    Tcg,
}

impl AccelMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccelMode::Kvm => "kvm",
            AccelMode::Tcg => "tcg",
        }
    }
}

impl std::fmt::Display for AccelMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "qemulbench",
    author,
    version,
    about = "Standalone self-contained cross-platform Linux benchmark orchestrator"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute commands directly in native rootfs via chroot
    Native {
        /// Mount host directory into rootfs [HOST_PATH[:GUEST_PATH]]
        #[arg(short = 'm', long, value_name = "HOST[:GUEST]", action = clap::ArgAction::Append)]
        mount: Vec<String>,
        /// Command and arguments to execute inside rootfs
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Execute cross-architecture binaries via QEMU user mode
    User {
        /// Target architecture: x86_64 or aarch64
        arch: Architecture,
        /// Mount/overlay host directory into rootfs [HOST_PATH[:GUEST_PATH]]
        #[arg(short = 'm', long, value_name = "HOST[:GUEST]", action = clap::ArgAction::Append)]
        mount: Vec<String>,
        /// Command and arguments to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Execute commands inside full-system QEMU virtual machine
    System {
        /// Target architecture: x86_64 or aarch64
        arch: Option<Architecture>,
        /// Acceleration engine: kvm or tcg
        accel: Option<AccelMode>,
        /// List detected ARM CPU clusters / parts on host and exit
        #[arg(long)]
        list_cpu_parts: bool,
        /// Mount host directory into guest VM [HOST_PATH[:GUEST_PATH]]
        #[arg(short = 'm', long, value_name = "HOST[:GUEST]", action = clap::ArgAction::Append)]
        mount: Vec<String>,
        /// Disable CPU topology detection and core affinity pinning
        #[arg(long)]
        no_cpu_topo: bool,
        /// Pre-select CPU part index (1-based) on heterogeneous ARM CPUs
        #[arg(long)]
        cpu_part: Option<usize>,
        /// Number of CPU cores / vCPUs to allocate (0 to disable default -smp for --qemu custom config)
        #[arg(long)]
        cpu: Option<String>,
        /// Custom memory size (0 to disable default -m for --qemu custom config, defaults to 1024M)
        #[arg(long)]
        mem: Option<String>,
        /// Extra QEMU arguments override passed directly to QEMU
        #[arg(long, value_name = "ARG", action = clap::ArgAction::Append, allow_hyphen_values = true)]
        qemu: Vec<String>,
        /// Command and arguments to execute inside guest VM after boot
        #[arg(last = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
}
