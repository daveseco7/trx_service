use crate::trx_engine::account::Account;
use crate::trx_engine::errors::EngineError;
use crate::trx_engine::transaction::{Input, State, Transaction, Type};
use anyhow::anyhow;
use std::collections::HashMap;

pub(crate) struct Ledger {
    accounts: HashMap<u16, Account>,
    trx: HashMap<u32, Transaction>,
}

impl Ledger {
    pub(crate) fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            trx: HashMap::new(),
        }
    }

    pub(crate) fn get_accounts(self) -> HashMap<u16, Account> {
        self.accounts
    }

    pub(crate) fn process_trx(&mut self, input: &Input) -> anyhow::Result<()> {
        // fetch account or create new record
        let account = self
            .accounts
            .entry(input.client)
            .or_insert(Account::new(input.client));

        match input.transaction_type {
            Type::Deposit => {
                // if this transaction is already present in the ledger there is an inconsistent behaviour.
                if self.trx.contains_key(&input.tx) {
                    return Err(anyhow!(EngineError::TrxAlreadyProcessed));
                }

                // validate that the amount has a workable value.
                let Some(amount) = input.amount else {
                    return Err(anyhow!(EngineError::TrxInvalidAmount));
                };

                account.deposit(amount)?;
                self.trx.insert(input.tx, Transaction::new(input));
            }

            Type::Withdrawal => {
                // if this transaction is already present in the ledger there is an inconsistent behaviour.
                if self.trx.contains_key(&input.tx) {
                    return Err(anyhow!(EngineError::TrxAlreadyProcessed));
                }

                // validate that the amount has a workable value.
                let Some(amount) = input.amount else {
                    return Err(anyhow!(EngineError::TrxInvalidAmount));
                };

                account.withdrawal(amount)?;
                self.trx.insert(input.tx, Transaction::new(input));
            }

            Type::Dispute => {
                // find the transaction to be disputed and if no transaction is found,
                // assume error from the banking partner. Continue processing the rest of the CSV.
                let disputed_trx = match self.trx.get_mut(&input.tx) {
                    Some(trx) => trx,
                    None => return Err(anyhow!(EngineError::TrxNotFound)),
                };

                // validate that the retrieved transaction belongs to the same client.
                if input.client != disputed_trx.client {
                    return Err(anyhow!(EngineError::TrxClientIdInconsistency));
                }

                // validate that the transaction to be disputed is a deposit or a withdrawal
                match disputed_trx.transaction_type {
                    Type::Withdrawal | Type::Resolve | Type::Dispute | Type::Chargeback  => {
                        return Err(anyhow!(EngineError::TrxNotDisputable))
                    }
                    _ => {}
                }

                // validate that the transaction to be disputed is not already under dispute or if it was chargeback.
                if disputed_trx.state != State::Ok {
                    return Err(anyhow!(EngineError::TrxNotInDisputableState));
                }

                // validate that the amount of the disputed trx has a workable value.
                let Some(amount) = disputed_trx.amount else {
                    return Err(anyhow!(EngineError::TrxInvalidAmount));
                };

                account.dispute(amount)?;
                // mark transaction as being disputed.
                disputed_trx.open_dispute();
            }

            Type::Resolve => {
                // find the transaction to be resolved and if no transaction is found,
                // assume error from the banking partner. Continue processing the rest of the CSV.
                let resolved_trx = match self.trx.get_mut(&input.tx) {
                    Some(trx) => trx,
                    None => return Err(anyhow!(EngineError::TrxNotFound)),
                };

                // validate that the transaction to be resolved is under dispute.
                if resolved_trx.state != State::Disputed {
                    return Err(anyhow!(EngineError::TrxNotInDispute));
                }

                // validate that the retrieved transaction belongs to the same client.
                if input.client != resolved_trx.client {
                    return Err(anyhow!(EngineError::TrxClientIdInconsistency));
                }

                // validate that the amount of the resolved trx has a workable value.
                let Some(amount) = resolved_trx.amount else {
                    return Err(anyhow!(EngineError::TrxInvalidAmount));
                };

                account.resolve(amount)?;
                // mark disputed transaction as resolved.
                resolved_trx.resolve_dispute();
            }

            Type::Chargeback => {
                // find the transaction to be chargeback and if no transaction is found,
                // assume error from the banking partner. Continue processing the rest of the CSV.
                let chargeback_trx = match self.trx.get_mut(&input.tx) {
                    Some(trx) => trx,
                    None => return Err(anyhow!(EngineError::TrxNotFound)),
                };

                // validate that the retrieved transaction belongs to the same client.
                if input.client != chargeback_trx.client {
                    return Err(anyhow!(EngineError::TrxClientIdInconsistency));
                }

                // validate that the amount of the chargeback trx has a workable value.
                let Some(amount) = chargeback_trx.amount else {
                    return Err(anyhow!(EngineError::TrxInvalidAmount));
                };

                // perform the necessary calculations for chargeback and lock account.
                account.chargeback(amount)?;

                // mark transaction as chargeback.
                chargeback_trx.chargeback_dispute();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::ops::Neg;

    /// helper func to provide an input fixture to use in the tests
    fn input(trx_type: Type, client: u16, tx: u32, amount: Option<Decimal>) -> Input {
        Input {
            transaction_type: trx_type,
            client,
            tx,
            amount,
        }
    }

    #[test]
    fn process_trx_successful_deposit_dispute_resolve_chargeback() {
        let client_id = 0;
        let tx_id = 1;
        let amount = dec!(1500);

        let trxs = vec![
            input(Type::Deposit, client_id, tx_id, Some(amount)),
            input(Type::Dispute, client_id, tx_id, None),
            input(Type::Resolve, client_id, tx_id, None),
            input(Type::Chargeback, client_id, tx_id, None),
        ];

        let mut ledger = Ledger::new();

        for t in trxs.into_iter() {
            _ = ledger.process_trx(&t)
        }

        let account = ledger.accounts.get(&client_id).expect("account not found");
        assert_eq!(account.client, client_id);
        assert_eq!(account.available, amount);
        assert_eq!(account.held, amount.neg());
        assert_eq!(account.total, dec!(0));
        assert!(account.locked);
    }

    #[test]
    fn process_trx_successful_after_dispute_resolve() {
        let client_id = 0;
        let tx = 1;
        let amount_deposit = dec!(1500);
        let amount_withdrawal = dec!(500);

        let trxs = vec![
            input(Type::Deposit, client_id, tx, Some(amount_deposit)),
            input(Type::Withdrawal, client_id, 2, Some(amount_withdrawal)),
            input(Type::Dispute, client_id, tx, None),
            input(Type::Resolve, client_id, tx, None),
            input(Type::Deposit, client_id, 3, Some(amount_deposit)),
            input(Type::Withdrawal, client_id, 4, Some(amount_withdrawal)),
        ];

        let mut ledger = Ledger::new();

        for trx in trxs.into_iter() {
            _ = ledger.process_trx(&trx)
        }

        let account = ledger.accounts.get(&client_id).expect("account not found");
        assert_eq!(account.client, client_id);
        assert_eq!(account.available, dec!(2000));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(2000));
        assert!(!account.locked);

        assert_eq!(4, ledger.trx.len());
        let trx1 = ledger.trx.get(&tx).expect("transaction 1 not found");
        assert_eq!(trx1.amount, Some(amount_deposit));
        assert_eq!(trx1.state, State::Ok);
        assert_eq!(trx1.client, client_id);

        let trx2 = ledger.trx.get(&2).expect("transaction 2 not found");
        assert_eq!(trx2.amount, Some(amount_withdrawal));
        assert_eq!(trx2.state, State::Ok);
        assert_eq!(trx2.client, client_id);

        let trx3 = ledger.trx.get(&3).expect("transaction 3 not found");
        assert_eq!(trx3.amount, Some(amount_deposit));
        assert_eq!(trx3.state, State::Ok);
        assert_eq!(trx3.client, client_id);

        let trx4 = ledger.trx.get(&4).expect("transaction 4 not found");
        assert_eq!(trx4.amount, Some(amount_withdrawal));
        assert_eq!(trx4.state, State::Ok);
        assert_eq!(trx4.client, client_id);
    }

    #[test]
    fn process_trx_ignore_all_operations_after_chargeback() {
        let client_id = 0;
        let tx_id = 1;
        let amount = dec!(1500);

        let trxs = vec![
            input(Type::Deposit, client_id, tx_id, Some(amount)),
            input(Type::Dispute, client_id, tx_id, None),
            input(Type::Chargeback, client_id, tx_id, None),
            input(Type::Deposit, client_id, 2, Some(amount)),
            input(Type::Withdrawal, client_id, 3, Some(amount)),
            input(Type::Dispute, client_id, 4, None),
            input(Type::Resolve, client_id, 5, None),
        ];

        let mut ledger = Ledger::new();

        for t in trxs.into_iter() {
            _ = ledger.process_trx(&t)
        }

        let account = ledger.accounts.get(&client_id).expect("account not found");
        assert_eq!(account.client, client_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(account.locked);

        assert_eq!(1, ledger.trx.len());
        let trx = ledger.trx.get(&tx_id).expect("transaction not found");
        assert_eq!(trx.amount, Some(amount));
        assert_eq!(trx.state, State::Chargeback);
        assert_eq!(trx.client, client_id);
    }

    #[test]
    fn process_trx_deposit_fail_when_trx_already_processed() {
        let tx = 123456789;
        let client = 1234;
        let amount = Some(dec!(1500));
        let input = input(Type::Deposit, client, tx, amount);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        // validate that preconditions are verified
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client));

        // repeat the same transaction to assert the expected behaviour
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxAlreadyProcessed.to_string()
        );
    }

    #[test]
    fn process_trx_deposit_fail_when_amount_is_not_present() {
        let tx = 123456789;
        let client = 1234;
        let amount = None;
        let input = input(Type::Deposit, client, tx, amount);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxInvalidAmount.to_string()
        );
    }

    #[test]
    fn process_trx_deposit_fail_when_account_method_returns_error() {
        let tx = 123456789;
        let client = 1234;
        let amount = Some(dec!(-1500));
        let input = input(Type::Deposit, client, tx, amount);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::NegativeAmount.to_string()
        );
    }

    #[test]
    fn process_trx_withdrawal_fail_when_trx_already_processed() {
        let tx_deposit = 123;
        let tx = 123456789;
        let client = 1234;
        let amount = Some(dec!(1500));
        let input_deposit = input(Type::Deposit, client, tx_deposit, amount);
        let input_withdrawal = input(Type::Withdrawal, client, tx, amount);

        let mut ledger = Ledger::new();
        // load balance in the account, so it can be withdrawal
        let result = ledger.process_trx(&input_deposit);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx_deposit));
        assert!(ledger.accounts.contains_key(&client));

        let result = ledger.process_trx(&input_withdrawal);
        // validate that preconditions are verified (withdrawal was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client));

        // repeat the same transaction to assert the expected behaviour
        let result = ledger.process_trx(&input_withdrawal);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxAlreadyProcessed.to_string()
        );
    }

    #[test]
    fn process_trx_withdrawal_fail_when_amount_is_not_present() {
        let tx = 123456789;
        let client = 1234;
        let amount = None;
        let input = input(Type::Withdrawal, client, tx, amount);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxInvalidAmount.to_string()
        );
    }

    #[test]
    fn process_trx_withdrawal_fail_when_account_method_returns_error() {
        let tx = 123456789;
        let client = 1234;
        let amount = Some(dec!(-1500));
        let input = input(Type::Withdrawal, client, tx, amount);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::NegativeAmount.to_string()
        );
    }

    #[test]
    fn process_trx_dispute_fail_when_disputed_trx_not_found() {
        let tx = 123456789;
        let client = 1234;
        let input = input(Type::Dispute, client, tx, None);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxNotFound.to_string()
        );
    }

    #[test]
    fn process_trx_dispute_fail_when_disputed_trx_belongs_to_another_user() {
        let tx = 123456789;
        let client_deposit = 1;
        let client = 1234;
        let amount = Some(dec!(1500));
        let input_deposit = input(Type::Deposit, client_deposit, tx, amount);
        let input_dispute = input(Type::Dispute, client, tx, None);

        let mut ledger = Ledger::new();
        // load balance in the account, so it can be disputed
        let result = ledger.process_trx(&input_deposit);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client_deposit));

        let result = ledger.process_trx(&input_dispute);
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxClientIdInconsistency.to_string()
        );
    }

    #[test]
    fn process_trx_dispute_fail_when_transaction_type_not_valid() {
        let invalid_inputs = [
            input(Type::Withdrawal, 0, 0, Some(dec!(10.0))),
            input(Type::Dispute, 0, 1, None),
            input(Type::Resolve, 0, 2, None),
            input(Type::Chargeback, 0, 3, None),
        ];

        for (index, invalid_input) in invalid_inputs.iter().enumerate() {
            let index = index
                .to_u32()
                .expect("error while converting from usize to u32");

            // build pre-conditions but inserting a erroneous transaction in the ledger
            let trx = Transaction::new(invalid_input);
            let mut ledger = Ledger::new();
            ledger.trx.insert(index, trx);

            let input_dispute = input(Type::Dispute, 0, index, None);
            let result = ledger.process_trx(&input_dispute);
            assert!(result.is_err());
            assert_eq!(
                format!("{}", result.unwrap_err()),
                EngineError::TrxNotDisputable.to_string()
            );
        }
    }

    #[test]
    fn process_trx_dispute_fail_when_transaction_to_be_disputed_is_not_in_a_disputable_state() {
        let input_invalid = input(Type::Deposit, 0, 1, None);
        let input_dispute = input(Type::Dispute, 0, 1, None);

        // build pre-conditions but inserting a erroneous transaction in the ledger
        let mut trx = Transaction::new(&input_invalid);
        trx.state = State::Chargeback;

        let mut ledger = Ledger::new();
        ledger.trx.insert(1, trx);

        let result = ledger.process_trx(&input_dispute);
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxNotInDisputableState.to_string()
        );
    }
    #[test]
    fn process_trx_resolve_fail_when_disputed_trx_not_found() {
        let tx = 123456789;
        let client = 1234;
        let input = input(Type::Resolve, client, tx, None);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxNotFound.to_string()
        );
    }

    #[test]
    fn process_trx_resolve_fail_when_resolved_trx_not_in_dispute_state() {
        let tx = 123456789;
        let client = 1234;
        let amount = Some(dec!(1500));
        let input_deposit = input(Type::Deposit, client, tx, amount);
        let input_resolve = input(Type::Resolve, client, tx, None);

        let mut ledger = Ledger::new();
        // load balance in the account, so it can be disputed
        let result = ledger.process_trx(&input_deposit);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client));

        let result = ledger.process_trx(&input_resolve);
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxNotInDispute.to_string()
        );
    }

    #[test]
    fn process_trx_resolve_fail_when_resolved_trx_belongs_to_another_user() {
        let tx = 123456789;
        let client = 1234;
        let client_deposit = 1;
        let amount = Some(dec!(1500));
        let input_deposit = input(Type::Deposit, client_deposit, tx, amount);
        let input_dispute = input(Type::Dispute, client_deposit, tx, None);
        let input_resolve = input(Type::Resolve, client, tx, None);

        let mut ledger = Ledger::new();
        // load balance in the account, so it can be disputed
        let result = ledger.process_trx(&input_deposit);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client_deposit));

        // dispute transaction, so it can be resolved
        let result = ledger.process_trx(&input_dispute);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client_deposit));

        let result = ledger.process_trx(&input_resolve);
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxClientIdInconsistency.to_string()
        );
    }

    #[test]
    fn process_trx_chargeback_fail_when_disputed_trx_not_found() {
        let tx = 123456789;
        let client = 1234;
        let input = input(Type::Chargeback, client, tx, None);

        let mut ledger = Ledger::new();
        let result = ledger.process_trx(&input);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxNotFound.to_string()
        );
    }

    #[test]
    fn process_trx_chargeback_fail_when_chargeback_trx_belongs_to_another_user() {
        let tx = 123456789;
        let client = 1234;
        let client_deposit = 1;
        let amount = Some(dec!(1500));
        let input_deposit = input(Type::Deposit, client_deposit, tx, amount);
        let input_dispute = input(Type::Dispute, client_deposit, tx, None);
        let input_resolve = input(Type::Resolve, client_deposit, tx, None);
        let input_chargeback = input(Type::Chargeback, client, tx, None);

        let mut ledger = Ledger::new();
        // load balance in the account, so it can be disputed
        let result = ledger.process_trx(&input_deposit);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client_deposit));

        // dispute transaction, so it can be resolved
        let result = ledger.process_trx(&input_dispute);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client_deposit));

        // resolve transaction, so it can be chargeback
        let result = ledger.process_trx(&input_resolve);
        // validate that preconditions are verified (deposit was successful)
        assert!(result.is_ok());
        assert!(ledger.trx.contains_key(&tx));
        assert!(ledger.accounts.contains_key(&client_deposit));

        let result = ledger.process_trx(&input_chargeback);
        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::TrxClientIdInconsistency.to_string()
        );
    }
}
