use crate::trx_engine::ledger::Ledger;
use log::{info, warn};
use std::io::{Read, Write};

pub fn process_transactions_file<T: Read, U: Write>(
    mut rdr: csv::Reader<T>,
    writer: U,
) -> anyhow::Result<()> {
    let mut ledger = Ledger::new();

    for result in rdr.deserialize() {
        let trx_input = match result {
            Ok(input) => input,
            Err(e) => {
                info!("failed to parse input from csv: {:?}", e);

                // ignore lines with parsing errors.
                continue;
            }
        };

        match ledger.process_trx(&trx_input) {
            Ok(_) => {}
            Err(e) => {
                warn!(
                    "failed to execute transaction: {:?} with error: {:?}",
                    trx_input, e
                );

                // ignore inputs with business logic errors.
                continue;
            }
        }
    }

    // write result to the provided writer.
    let mut output = csv::Writer::from_writer(writer);
    ledger
        .get_accounts()
        .iter()
        //.try_for_each(|(_, account)| output.serialize(account.format_account_precision_of_decimals_for_report()))?;
        .try_for_each(|(_, account)| output.serialize(account))?;

    Ok(())
}
