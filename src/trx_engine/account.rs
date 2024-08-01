use crate::trx_engine::errors::EngineError;
use anyhow::anyhow;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Account represents an account of a client.
/// All operations that mutate an Account should be done through the provided methods.
/// The output result is achieved by using the serde serializer.
/// No business logic is validated in this op wrapper (for example calling a dispute on a deposit)
/// that is up to the consumer (ledger) to ensure.
#[derive(Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Account {
    #[serde(rename = "client")]
    pub client: u16,

    #[serde(rename = "available")]
    pub(crate) available: Decimal,

    #[serde(rename = "held")]
    pub(crate) held: Decimal,

    #[serde(rename = "total")]
    pub(crate) total: Decimal,

    #[serde(rename = "locked")]
    pub(crate) locked: bool,
}

impl Account {
    pub(crate) fn new(id: u16) -> Self {
        Self {
            client: id,
            available: dec!(0.0),
            held: dec!(0.0),
            total: dec!(0.0),
            locked: false,
        }
    }

    /*pub(crate) fn format_account_precision_of_decimals_for_report(&self) -> Self {
        Self {
            client: self.client,
            available: self.available.round_dp(4),
            held: self.held.round_dp(4),
            total: self.total.round_dp(4),
            locked: self.locked,
        }
    }*/

    pub(crate) fn deposit(&mut self, amount: Decimal) -> anyhow::Result<()> {
        is_amount_negative(&amount)?;

        self.is_account_locked()?;

        self.available += amount;
        self.total += amount;

        Ok(())
    }

    pub(crate) fn withdrawal(&mut self, amount: Decimal) -> anyhow::Result<()> {
        is_amount_negative(&amount)?;

        self.is_account_locked()?;

        // if the available balance is not enough for the withdrawal
        // return error and do not perform operation.
        if amount > self.available {
            return Err(anyhow!(EngineError::InsufficientFunds));
        }

        self.available -= amount;
        self.total -= amount;

        Ok(())
    }

    pub(crate) fn dispute(&mut self, amount: Decimal) -> anyhow::Result<()> {
        is_amount_negative(&amount)?;

        self.is_account_locked()?;

        self.available -= amount;
        self.held += amount;

        Ok(())
    }

    pub(crate) fn resolve(&mut self, amount: Decimal) -> anyhow::Result<()> {
        is_amount_negative(&amount)?;

        self.is_account_locked()?;

        self.available += amount;
        self.held -= amount;

        Ok(())
    }

    pub(crate) fn chargeback(&mut self, amount: Decimal) -> anyhow::Result<()> {
        is_amount_negative(&amount)?;

        self.is_account_locked()?;

        self.total -= amount;
        self.held -= amount;
        self.locked = true;

        Ok(())
    }

    fn is_account_locked(&self) -> anyhow::Result<()> {
        if self.locked {
            return Err(anyhow!(EngineError::AccountLocked));
        }

        Ok(())
    }
}

/// Helper function to validate if an amount.
/// # Errors
/// * An error is returned if the amount provided is negative.
///
fn is_amount_negative(amount: &Decimal) -> anyhow::Result<()> {
    if amount.is_sign_negative() {
        return Err(anyhow!(EngineError::NegativeAmount));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn is_amount_negative_when_provided_positive() {
        let positive: Decimal = dec!(1234);
        let result = is_amount_negative(&positive);

        assert!(result.is_ok());
    }

    #[test]
    fn is_amount_negative_when_provided_negative() {
        let positive: Decimal = dec!(-123);
        let result = is_amount_negative(&positive);

        assert!(result.is_err());
    }

    #[test]
    fn account_new() {
        let account_id: u16 = 1234;
        let account = Account::new(account_id);

        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(!account.locked)
    }

    #[test]
    fn account_deposit_successful() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        let deposit_amount: Decimal = dec!(1500);
        let result = account.deposit(deposit_amount);

        assert!(result.is_ok());
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, deposit_amount);
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, deposit_amount);
        assert!(!account.locked);
    }

    #[test]
    fn account_deposit_fail_when_provided_negative_amount() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        let deposit_amount: Decimal = dec!(-1500);
        let result = account.deposit(deposit_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::NegativeAmount.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(!account.locked);
    }

    #[test]
    fn account_deposit_fail_when_account_is_locked() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);
        // mutate private field to cater to the test. This should not be possible with the public API.
        account.locked = true;

        let deposit_amount: Decimal = dec!(1500);
        let result = account.deposit(deposit_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::AccountLocked.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(account.locked);
    }

    #[test]
    fn account_withdrawal_successful() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        // adding have balance to enable withdrawal
        let deposit_amount: Decimal = dec!(1500);
        account.deposit(deposit_amount).expect("failed to deposit");

        let withdrawal_amount: Decimal = dec!(500);
        let result = account.withdrawal(withdrawal_amount);

        assert!(result.is_ok());
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, deposit_amount - withdrawal_amount);
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, deposit_amount - withdrawal_amount);
        assert!(!account.locked);
    }

    #[test]
    fn account_withdrawal_fail_when_provided_negative_amount() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        let withdrawal_amount: Decimal = dec!(-500);
        let result = account.withdrawal(withdrawal_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::NegativeAmount.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(!account.locked);
    }

    #[test]
    fn account_withdrawal_fail_when_account_is_locked() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);
        // mutate private field to cater to the test. This should not be possible with the public API.
        account.locked = true;

        let withdrawal_amount: Decimal = dec!(500);
        let result = account.withdrawal(withdrawal_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::AccountLocked.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(account.locked);
    }

    #[test]
    fn account_withdrawal_fail_when_insufficient_funds() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        let withdrawal_amount: Decimal = dec!(500);
        let result = account.withdrawal(withdrawal_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::InsufficientFunds.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(!account.locked);
    }

    #[test]
    fn account_dispute_successful() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        // adding have balance to enable withdrawal
        let deposit_amount: Decimal = dec!(1500);
        account.deposit(deposit_amount).expect("failed to deposit");

        let dispute_amount: Decimal = dec!(500);
        let result = account.dispute(dispute_amount);

        assert!(result.is_ok());
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, deposit_amount - dispute_amount);
        assert_eq!(account.held, dispute_amount);
        assert_eq!(account.total, deposit_amount);
        assert!(!account.locked);
    }

    #[test]
    fn account_dispute_fail_when_provided_negative_amount() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        let dispute_amount: Decimal = dec!(-500);
        let result = account.dispute(dispute_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::NegativeAmount.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(!account.locked);
    }

    #[test]
    fn account_dispute_fail_when_account_is_locked() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);
        // mutate private field to cater to the test. This should not be possible with the public API.
        account.locked = true;

        let dispute_amount: Decimal = dec!(500);
        let result = account.dispute(dispute_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::AccountLocked.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(account.locked);
    }

    #[test]
    fn account_resolve_successful() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        // adding have balance to enable withdrawal
        let deposit_amount: Decimal = dec!(1500);
        account.deposit(deposit_amount).expect("failed to deposit");
        // disputing a transaction to enable resolve
        let dispute_amount: Decimal = dec!(500);
        account.dispute(dispute_amount).expect("failed to dispute");

        let resolve_amount: Decimal = dec!(500);
        let result = account.resolve(resolve_amount);

        assert!(result.is_ok());
        assert_eq!(account.client, account_id);
        assert_eq!(
            account.available,
            deposit_amount - dispute_amount + resolve_amount
        );
        assert_eq!(account.held, dispute_amount - resolve_amount);
        assert_eq!(account.total, deposit_amount);
        assert!(!account.locked);
    }

    #[test]
    fn account_resolve_fail_when_provided_negative_amount() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        let resolve_amount: Decimal = dec!(-500);
        let result = account.resolve(resolve_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::NegativeAmount.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(!account.locked);
    }

    #[test]
    fn account_resolve_fail_when_account_is_locked() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);
        // mutate private field to cater to the test. This should not be possible with the public API.
        account.locked = true;

        let resolve_amount: Decimal = dec!(500);
        let result = account.resolve(resolve_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::AccountLocked.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(account.locked);
    }

    #[test]
    fn account_chargeback_successful() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        // adding have balance to enable withdrawal
        let deposit_amount: Decimal = dec!(1500);
        account.deposit(deposit_amount).expect("failed to deposit");
        // disputing a transaction to enable resolve
        let dispute_amount: Decimal = dec!(500);
        account.dispute(dispute_amount).expect("failed to dispute");
        // resolving a transaction to enable a chargeback
        let resolve_amount: Decimal = dec!(500);
        account.resolve(resolve_amount).expect("failed to resolve");

        let chargeback_amount: Decimal = dec!(500);
        let result = account.chargeback(chargeback_amount);

        assert!(result.is_ok());
        assert_eq!(account.client, account_id);
        assert_eq!(
            account.available,
            deposit_amount - dispute_amount + resolve_amount
        );
        assert_eq!(
            account.held,
            dispute_amount - resolve_amount - chargeback_amount
        );
        assert_eq!(account.total, deposit_amount - chargeback_amount);
        assert!(account.locked);
    }

    #[test]
    fn account_chargeback_fail_when_provided_negative_amount() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);

        let resolve_amount: Decimal = dec!(-500);
        let result = account.chargeback(resolve_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::NegativeAmount.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
    }

    #[test]
    fn account_chargeback_fail_when_account_is_locked() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);
        // mutate private field to cater to the test. This should not be possible with the public API.
        account.locked = true;

        let resolve_amount: Decimal = dec!(500);
        let result = account.chargeback(resolve_amount);

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::AccountLocked.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
    }

    #[test]
    fn account_is_account_locked_when_account_locked() {
        let account_id: u16 = 1234;
        let mut account = Account::new(account_id);
        account.locked = true;

        let result = account.is_account_locked();

        assert!(result.is_err());
        assert_eq!(
            format!("{}", result.unwrap_err()),
            EngineError::AccountLocked.to_string()
        );
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(account.locked);
    }

    #[test]
    fn account_is_account_locked_when_account_not_locked() {
        let account_id: u16 = 1234;
        let account = Account::new(account_id);

        let result = account.is_account_locked();

        assert!(result.is_ok());
        assert_eq!(account.client, account_id);
        assert_eq!(account.available, dec!(0));
        assert_eq!(account.held, dec!(0));
        assert_eq!(account.total, dec!(0));
        assert!(!account.locked);
    }
}
