// Tokenized Vault - ERC-4626-style vault implementation on Solana
// Security: Follows Solana security best practices with comprehensive validation
// Architecture: Registry + Whitelist (Option 3) for protocol management

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("VAULTvgMLuVNhWKYA2oYzH5gcz6XxsjXrqvnxTJbG8F");

#[program]
pub mod tokenized_vault {
    use super::*;

    /// Initialize a new vault for a given asset token
    ///
    /// Security considerations:
    /// - Validates authority is signer
    /// - Initializes vault state with proper PDAs
    /// - Creates share mint with vault as mint authority
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    /// Deposit assets into the vault and receive shares
    ///
    /// Security considerations:
    /// - Validates user token accounts (mint, owner)
    /// - Uses checked math for share calculation
    /// - Follows checks-effects-interactions pattern
    /// - Emits event for tracking
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    /// Invest vault assets into a whitelisted protocol via CPI
    ///
    /// Architecture: Validates target against protocol registry whitelist
    /// Security considerations:
    /// - Authority-only function (has_one constraint)
    /// - Validates target against approved protocol registry
    /// - Tracks invested amount per protocol
    /// - Prevents rug pulls by restricting investment destinations
    /// - Emits event for transparency
    pub fn invest(ctx: Context<Invest>, amount: u64) -> Result<()> {
        instructions::invest::handler(ctx, amount)
    }

    /// Add a new protocol to the approved whitelist
    ///
    /// Security considerations:
    /// - Authority-only function
    /// - Validates protocol doesn't already exist
    /// - Enforces registry size limits
    /// - Emits event for tracking
    pub fn add_protocol(
        ctx: Context<AddProtocol>,
        target: Pubkey,
        name: String,
    ) -> Result<()> {
        instructions::add_protocol::handler(ctx, target, name)
    }

    /// Toggle a protocol on/off in the whitelist
    ///
    /// Security considerations:
    /// - Authority-only function
    /// - Allows disabling protocols without removing them
    /// - Emergency shutdown capability per protocol
    /// - Emits event for tracking
    pub fn toggle_protocol(
        ctx: Context<ToggleProtocol>,
        target: Pubkey,
        enabled: bool,
    ) -> Result<()> {
        instructions::toggle_protocol::handler(ctx, target, enabled)
    }
}

