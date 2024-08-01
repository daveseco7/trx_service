use rust_decimal::Decimal;

#[derive(Debug, serde::Deserialize, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Type {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, serde::Deserialize, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum State {
    Ok,
    Disputed,
    Chargeback,
}

/// Input represents a line of the provided input (csv arg from the CLI).
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Input {
    #[serde(rename = "type")]
    pub(crate) transaction_type: Type,

    #[serde(rename = "client")]
    pub(crate) client: u16,

    #[serde(rename = "tx")]
    pub(crate) tx: u32,

    #[serde(deserialize_with = "csv::invalid_option")]
    #[serde(rename = "amount")]
    pub(crate) amount: Option<Decimal>,
}

/// Transaction represents a business translation from an input line.
/// All operations that mutate a transaction should be done through the provided methods.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Transaction {
    #[serde(rename = "type")]
    pub(crate) transaction_type: Type,

    #[serde(rename = "client")]
    pub(crate) client: u16,

    #[serde(rename = "amount")]
    pub(crate) amount: Option<Decimal>,

    #[serde(rename = "state")]
    pub(crate) state: State,
}

impl Transaction {
    pub(crate) fn new(input: &Input) -> Self {
        Self {
            transaction_type: input.transaction_type,
            client: input.client,
            amount: input.amount,
            state: State::Ok,
        }
    }

    pub(crate) fn open_dispute(&mut self) {
        self.state = State::Disputed
    }

    pub(crate) fn resolve_dispute(&mut self) {
        self.state = State::Ok
    }

    pub(crate) fn chargeback_dispute(&mut self) {
        self.state = State::Chargeback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// helper func to provide an input fixture to use in the tests
    fn input(amount: Option<Decimal>) -> Input {
        Input {
            transaction_type: Type::Deposit,
            client: 1234,
            tx: 123456789,
            amount,
        }
    }

    #[test]
    fn transaction_new_when_input_has_amount() {
        let input = input(Some(dec!(1500)));
        let transaction = Transaction::new(&input);

        assert_eq!(transaction.transaction_type, input.transaction_type);
        assert_eq!(transaction.client, input.client);
        assert_eq!(transaction.amount, input.amount);
        assert_eq!(transaction.state, State::Ok);
    }

    #[test]
    fn transaction_new_when_input_has_no_amount() {
        let input = input(None);
        let transaction = Transaction::new(&input);

        assert_eq!(transaction.transaction_type, input.transaction_type);
        assert_eq!(transaction.client, input.client);
        assert_eq!(transaction.amount, input.amount);
        assert_eq!(transaction.state, State::Ok);
    }

    #[test]
    fn transaction_open_dispute() {
        let input = input(None);
        let mut transaction = Transaction::new(&input);

        transaction.open_dispute();

        assert_eq!(transaction.client, input.client);
        assert_eq!(transaction.amount, input.amount);
        assert_eq!(transaction.state, State::Disputed);
    }

    #[test]
    fn transaction_resolve_dispute() {
        let input = input(None);
        let mut transaction = Transaction::new(&input);

        transaction.resolve_dispute();

        assert_eq!(transaction.client, input.client);
        assert_eq!(transaction.amount, input.amount);
        assert_eq!(transaction.state, State::Ok);
    }

    #[test]
    fn transaction_chargeback_dispute() {
        let input = input(None);
        let mut transaction = Transaction::new(&input);

        transaction.chargeback_dispute();

        assert_eq!(transaction.client, input.client);
        assert_eq!(transaction.amount, input.amount);
        assert_eq!(transaction.state, State::Chargeback);
    }
}
