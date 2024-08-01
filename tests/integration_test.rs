use csv::Trim::All;
use csv::{Reader, ReaderBuilder};
use std::collections::HashMap;
use std::io::{BufWriter, Read};
use trx_service::trx_engine;
use trx_service::trx_engine::account::Account;

#[test]
fn process_input_invalid() {
    const FILE_PATH: &str = "trx_invalid_format";

    let rdr = ReaderBuilder::new()
        .trim(All)
        .flexible(true)
        .from_path(format!("tests/{}.csv", FILE_PATH))
        .expect("failed to fixture file");

    let mut actual = Vec::new();
    let writer = BufWriter::new(&mut actual);

    trx_engine::processor::process_transactions_file(rdr, writer)
        .expect("failed read file to process");

    let mut actual_reader = ReaderBuilder::new().from_reader(actual.as_slice());
    let actual_accounts = parse_from_csv_to_accounts_map(&mut actual_reader);

    let mut expected_rdr = ReaderBuilder::new()
        .trim(All)
        .flexible(true)
        .from_path(format!("tests/{}_expected.csv", FILE_PATH))
        .expect("failed to fixture file");

    let expected_accounts = parse_from_csv_to_accounts_map(&mut expected_rdr);

    compare_actual_with_expectations(actual_accounts, expected_accounts)
}

#[test]
fn process_multiple_clients() {
    const FILE_PATH: &str = "multiple_clients";

    let rdr = ReaderBuilder::new()
        .trim(All)
        .flexible(true)
        .from_path(format!("tests/{}.csv", FILE_PATH))
        .expect("failed to fixture file");

    let mut actual = Vec::new();
    let writer = BufWriter::new(&mut actual);

    trx_engine::processor::process_transactions_file(rdr, writer)
        .expect("failed read file to process");

    let mut actual_reader = ReaderBuilder::new().from_reader(actual.as_slice());
    let actual_accounts = parse_from_csv_to_accounts_map(&mut actual_reader);

    let mut expected_rdr = ReaderBuilder::new()
        .trim(All)
        .flexible(true)
        .from_path(format!("tests/{}_expected.csv", FILE_PATH))
        .expect("failed to fixture file");

    let expected_accounts = parse_from_csv_to_accounts_map(&mut expected_rdr);

    compare_actual_with_expectations(actual_accounts, expected_accounts)
}

#[test]
fn process_after_dispute() {
    const FILE_PATH: &str = "process_after_dispute";

    let rdr = ReaderBuilder::new()
        .trim(All)
        .flexible(true)
        .from_path(format!("tests/{}.csv", FILE_PATH))
        .expect("failed to fixture file");

    let mut actual = Vec::new();
    let writer = BufWriter::new(&mut actual);

    trx_engine::processor::process_transactions_file(rdr, writer)
        .expect("failed read file to process");

    let mut actual_reader = ReaderBuilder::new().from_reader(actual.as_slice());
    let actual_accounts = parse_from_csv_to_accounts_map(&mut actual_reader);

    let mut expected_rdr = ReaderBuilder::new()
        .trim(All)
        .flexible(true)
        .from_path(format!("tests/{}_expected.csv", FILE_PATH))
        .expect("failed to fixture file");

    let expected_accounts = parse_from_csv_to_accounts_map(&mut expected_rdr);

    compare_actual_with_expectations(actual_accounts, expected_accounts)
}

// Helper func to compare account maps.
fn compare_actual_with_expectations(actual: HashMap<u16, Account>, expect: HashMap<u16, Account>) {
    assert_eq!(actual.len(), expect.len());

    for (key, actual_account) in actual.into_iter() {
        let expected_account = expect
            .get(&key)
            .expect("missing account entry in expectations");
        assert_eq!(actual_account, *expected_account);
    }
}

// Helper method to convert from a CSV in a Reader to a HashMap containing the accounts
fn parse_from_csv_to_accounts_map<R: Read>(reader: &mut Reader<R>) -> HashMap<u16, Account> {
    let mut accounts: HashMap<u16, Account> = HashMap::new();
    for account in reader.deserialize() {
        let account: Account = account.expect("failed to extract account");
        accounts.insert(account.client, account);
    }

    accounts
}
