#![warn(
    rust_2018_idioms,
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic
)]
#![allow(clippy::module_name_repetitions)]

use anyhow::Result;
use structopt::StructOpt as _;

mod cli;
mod error;
mod manifest;

use cli::Cargo;
use manifest::Manifest;

fn main() -> Result<()> {
    Cargo::from_args().execute()?;
    Ok(())
}
