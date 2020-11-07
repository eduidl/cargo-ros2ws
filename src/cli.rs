use std::path::PathBuf;

use anyhow::Result;
use structopt::{clap, StructOpt};

use crate::Manifest;

#[derive(Debug, StructOpt)]
#[structopt(about, bin_name("cargo"))]
#[structopt(setting(clap::AppSettings::ColoredHelp))]
pub struct Cargo {
    #[structopt(subcommand)]
    cmd: SubCommand,
}

impl Cargo {
    pub fn execute(self) -> Result<()> {
        match self.cmd {
            SubCommand::Clear(cmd) => cmd.execute(),
            SubCommand::AddMember(cmd) => cmd.execute(),
            SubCommand::AddPatch(cmd) => cmd.execute(),
        }?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
enum SubCommand {
    // Clear cargo workspace
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    Clear(CargoClear),

    /// Add a crate to the members of cargo workspace
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    AddMember(CargoAddMember),

    /// Override dependencies using [patch] section
    #[structopt(setting(clap::AppSettings::ColoredHelp))]
    AddPatch(CargoAddPatch),
}

#[derive(Debug, StructOpt)]
struct CargoClear {
    /// Absolute path to Cargo.toml of the cargo workspace
    #[structopt(short, long)]
    manifest_path: PathBuf,
}

impl CargoClear {
    fn execute(self) -> Result<()> {
        let manifest = Manifest::init();
        manifest.write_to(&self.manifest_path)?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
struct CargoAddMember {
    /// Absolute path to Cargo.toml of the cargo workspace
    #[structopt(short, long)]
    manifest_path: PathBuf,

    /// Absolute path to the crate to add to the members of cargo workspace
    #[structopt(long)]
    member: PathBuf,
}

impl CargoAddMember {
    fn execute(self) -> Result<()> {
        let mut manifest = Manifest::read_from(&self.manifest_path)?;
        manifest.add_member(self.member)?;
        manifest.write_to(&self.manifest_path)?;
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
struct CargoAddPatch {
    /// Absolute path to Cargo.toml of the cargo workspace
    #[structopt(short, long)]
    manifest_path: PathBuf,

    /// Crate name to patch
    #[structopt(short, long = "crate")]
    crate_name: String,

    /// Absolute path to the crate to override with
    #[structopt(short, long)]
    path: PathBuf,
}

impl CargoAddPatch {
    fn execute(self) -> Result<()> {
        let mut manifest = Manifest::read_from(&self.manifest_path)?;
        manifest.add_patch(&self.crate_name, self.path)?;
        manifest.write_to(&self.manifest_path)?;
        Ok(())
    }
}
