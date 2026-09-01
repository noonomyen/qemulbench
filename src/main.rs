mod assets;
mod cli;
mod executor;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Native { mount, cmd } => executor::native::run(&mount, &cmd),
        Commands::User { arch, mount, cmd } => executor::user::run(arch, &mount, &cmd),
        Commands::System {
            arch,
            accel,
            list_cpu_parts,
            mount,
            no_cpu_topo,
            cpu_part,
            cpu,
            mem,
            qemu,
            cmd,
        } => {
            if list_cpu_parts {
                utils::cpu::list_arm_clusters();
                return ExitCode::SUCCESS;
            }
            let (arch, accel) = match (arch, accel) {
                (Some(a), Some(m)) => (a, m),
                _ => {
                    eprintln!("error: <ARCH> and <ACCEL> are required for system mode (use --help for usage)");
                    return ExitCode::FAILURE;
                }
            };
            executor::system::run(executor::system::SystemOptions {
                arch,
                accel_mode: accel,
                mounts: &mount,
                no_cpu_topo,
                cpu_part,
                cpu: cpu.as_deref(),
                mem: mem.as_deref(),
                qemu_overrides: &qemu,
                cmd_args: &cmd,
            })
        }
    };

    match result {
        Ok(code) => {
            let clamped = if code < 0 { 1u8 } else { (code & 0xff) as u8 };
            ExitCode::from(clamped)
        }
        Err(e) => {
            eprintln!("Execution error: {}", e);
            ExitCode::FAILURE
        }
    }
}
