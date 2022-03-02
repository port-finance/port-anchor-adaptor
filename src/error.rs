use anchor_lang::prelude::*;

#[error_code]
pub enum PortAdaptorError {
    #[msg("Only has borrows, no collaterals")]
    Insolvency,
    #[msg("CollateralIndexOutOfBound")]
    CollateralIndexOutOfBound,
    #[msg("BorrowIndexOutOfBound")]
    BorrowIndexOutOfBound,
}
