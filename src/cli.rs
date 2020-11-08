use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Result};
use fs2::FileExt;
use structopt::{clap, StructOpt};

use crate::Manifest;

#[derive(Debug, StructOpt)]
#[structopt(about, bin_name("cargo-ros2ws"))]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub struct Cargo {
    #[structopt(flatten)]
    common_args: CommonArgs,

    #[structopt(subcommand)]
    cmd: SubCommand,
}

impl Cargo {
    pub fn execute(self) -> Result<()> {
        match self.cmd {
            SubCommand::AddMember(cmd) => cmd.execute(&self.common_args),
            SubCommand::AddPatch(cmd) => cmd.execute(&self.common_args),
        }?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
struct CommonArgs {
    /// Absolute path to Cargo.toml of the cargo workspace
    #[structopt(short, long)]
    manifest_path: PathBuf,

    /// Lock manifest file to process exclusively
    #[structopt(long)]
    with_lock: bool,

    /// How many seconds to wait for acquire lock (0 means forever)
    #[structopt(short = "s", long, default_value = "0")]
    wait_nsecs: u64,
}

struct FileLock {
    locking_file: Option<File>,
}

impl FileLock {
    fn from_cli_args(args: &CommonArgs) -> Result<Self> {
        if !args.with_lock {
            return Ok(Self { locking_file: None });
        }

        let file = File::open(&args.manifest_path)?;
        let deadline = Duration::from_secs(args.wait_nsecs);
        let timer = SystemTime::now();
        while args.wait_nsecs == 0 || timer.elapsed()? <= deadline {
            if file.try_lock_exclusive().is_ok() {
                return Ok(Self {
                    locking_file: Some(file),
                });
            }
        }

        Err(anyhow!(
            "Failed to aqcuire lock of file {}",
            args.manifest_path.display()
        ))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if let Some(ref file) = self.locking_file {
            file.unlock().unwrap()
        }
    }
}

#[derive(Debug, StructOpt)]
enum SubCommand {
    /// Add a crate to the members of cargo workspace
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    AddMember(CargoAddMember),

    /// Override dependencies using [patch] section
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    AddPatch(CargoAddPatch),
}

#[derive(Debug, StructOpt)]
struct CargoAddMember {
    /// Absolute path to the crate to add to the members of cargo workspace
    member: PathBuf,
}

impl CargoAddMember {
    fn execute(self, args: &CommonArgs) -> Result<()> {
        let _lock = FileLock::from_cli_args(args)?;

        let mut manifest = Manifest::read_from(&args.manifest_path)?;
        manifest.add_member(self.member)?;
        manifest.write_to(&args.manifest_path)?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
struct CargoAddPatch {
    /// Crate name to patch
    #[structopt(short, long = "crate")]
    crate_name: String,

    /// Absolute path to the crate to override with
    #[structopt(short, long)]
    path: PathBuf,
}

impl CargoAddPatch {
    fn execute(self, args: &CommonArgs) -> Result<()> {
        let _lock = FileLock::from_cli_args(args)?;

        let mut manifest = Manifest::read_from(&args.manifest_path)?;
        manifest.add_patch(&self.crate_name, self.path)?;
        manifest.write_to(&args.manifest_path)?;
        Ok(())
    }
}
