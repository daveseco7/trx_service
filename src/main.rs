use anyhow::{anyhow, Result};
use csv::Trim::All;
use env_logger::Env;
use log::error;
use std::{env, io};

use trx_service::trx_engine::processor;

//pub mod trx_engine;
fn main() -> Result<()> {
    // make logger configurable from env vars and default to info, if env vars are not provided.
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // as per the pdf, one argument is expected for the correct behaviour of the CLI.
    // if in the future more args are added, consider using CLAP for a fine grain control of validations and defaults.
    let filepath = match env::args().nth(1) {
        Some(file_path) => Ok(file_path),
        None => {
            error!("At least one argument is expected!");
            Err(anyhow!("At least one argument is expected!"))
        }
    }?;


    // create reader from the provided filepath and trim all whitespaces.
    let rdr = match csv::ReaderBuilder::new().trim(All).flexible(true).from_path(filepath) {
        Ok(rdr) => Ok(rdr),
        Err(err) => {
            error!("failed to read file: {}", err);
            Err(anyhow!("failed to read file"))
        }
    }?;
    
    let output = io::stdout();
    processor::process_transactions_file(rdr, output)

}
