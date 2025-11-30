use anchor_lang::prelude::*;

/// Custom error codes for the Tokenized Vault program
/// 
/// Security: Descriptive error messages without information leakage
#[error_code]
pub enum VaultError {
    #[msg("Deposit amount must be greater than zero")]
    ZeroDepositAmount,

    #[msg("Invest amount must be greater than zero")]
    ZeroInvestAmount,

    #[msg("Insufficient vault balance for investment")]
    InsufficientVaultBalance,

    #[msg("Math overflow occurred during calculation")]
    MathOverflow,

    #[msg("Cannot divide by zero - vault has no shares")]
    DivisionByZero,

    #[msg("Invalid token mint - does not match vault asset")]
    InvalidMint,

    #[msg("Invalid token account owner")]
    InvalidOwner,

    #[msg("Unauthorized - only vault authority can perform this action")]
    Unauthorized,

    #[msg("Invalid target program for investment")]
    InvalidTargetProgram,

    #[msg("Invest amount exceeds vault total assets")]
    InvestAmountTooLarge,

    #[msg("Protocol not approved - target not in whitelist or disabled")]
    ProtocolNotApproved,

    #[msg("Protocol already exists in registry")]
    ProtocolAlreadyExists,

    #[msg("Protocol not found in registry")]
    ProtocolNotFound,

    #[msg("Protocol registry is full - maximum protocols reached")]
    RegistryFull,

    #[msg("Protocol name too long - maximum 32 characters")]
    NameTooLong,
}

