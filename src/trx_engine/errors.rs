#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    InsufficientFunds,
    NegativeAmount,
    TrxAlreadyProcessed,
    TrxInvalidAmount,
    TrxNotFound,
    TrxNotInDisputableState,
    TrxNotInDispute,
    TrxNotDisputable,
    TrxClientIdInconsistency,
    AccountLocked,
}
impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientFunds => write!(f, "insufficient funds to execute transaction"),
            Self::NegativeAmount => write!(f, "negative transaction amount"),
            Self::TrxAlreadyProcessed => write!(f, "transaction already processed"),
            Self::TrxInvalidAmount => write!(f, "transaction contains an invalid amount to process"),
            Self::TrxNotFound => write!(f, "transaction not found in ledger"),
            Self::TrxNotInDisputableState => write!(f, "transaction not in a disputable state"),
            Self::TrxNotInDispute => write!(f, "transaction not in dispute"),
            Self::TrxNotDisputable => write!(f, "transaction type is not disputable"),
            Self::TrxClientIdInconsistency => write!(f, "client id present in transaction is not consistent with the related transaction"),
            Self::AccountLocked => write!(f, "account in locked state"),
        }
    }
}