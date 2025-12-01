use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::{constants::*, errors::*, events::*, state::*};

/// Invest vault assets into a whitelisted protocol via CPI
///
/// Architecture: Registry + Whitelist (Option 3)
/// - Validates target against on-chain protocol registry
/// - Prevents authority from investing to arbitrary addresses
/// - Tracks invested amount per protocol
///
#[derive(Accounts)]
pub struct Invest<'info> {
    /// Vault authority - only they can invest
    /// Security: Must be signer and match vault_state.authority
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Vault state PDA
    /// Security: has_one constraint validates authority from state
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.asset_mint.as_ref()],
        bump = vault_state.bump,
        has_one = authority @ VaultError::Unauthorized,
    )]
    pub vault_state: Account<'info, VaultState>,

    /// Protocol registry PDA
    /// Security: Validates target against whitelist
    #[account(
        mut,
        seeds = [b"protocol_registry", vault_state.key().as_ref()],
        bump = protocol_registry.bump,
    )]
    pub protocol_registry: Account<'info, ProtocolRegistry>,

    /// Vault authority PDA
    /// Security: CHECK constraint, validated by seeds
    /// CHECK: PDA used as authority, validated by seeds
    #[account(
        seeds = [VAULT_AUTHORITY_SEED, vault_state.asset_mint.as_ref()],
        bump = vault_state.authority_bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// Vault's token account (source of investment)
    /// Security: Must be correct mint and owned by vault_authority
    #[account(
        mut,
        constraint = vault_token_account.mint == vault_state.asset_mint @ VaultError::InvalidMint,
        constraint = vault_token_account.owner == vault_authority.key() @ VaultError::InvalidOwner,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    /// Target token account (destination for investment)
    /// Security: Must be correct mint, validated against whitelist
    #[account(
        mut,
        constraint = target_token_account.mint == vault_state.asset_mint @ VaultError::InvalidMint,
    )]
    pub target_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Invest>, amount: u64) -> Result<()> {
    // CHECKS: Validate amount and vault balance
    require!(amount > 0, VaultError::ZeroInvestAmount);

    let vault_state = &ctx.accounts.vault_state;
    let registry = &mut ctx.accounts.protocol_registry;
    let target = ctx.accounts.target_token_account.key();

    // CRITICAL SECURITY CHECK: Validate target is in whitelist
    require!(
        registry.is_protocol_approved(&target),
        VaultError::ProtocolNotApproved
    );

    // Verify vault has enough assets
    let available_balance = ctx.accounts.vault_token_account.amount;
    require!(
        available_balance >= amount,
        VaultError::InsufficientVaultBalance
    );

    // Additional safety: ensure invest amount doesn't exceed tracked total
    require!(
        amount <= vault_state.total_assets,
        VaultError::InvestAmountTooLarge
    );

    // Get protocol name for event
    let protocol_name = registry
        .get_protocol_mut(&target)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    // EFFECTS: Track investment in registry
    registry.track_investment(&target, amount)?;

    // INTERACTIONS: Perform CPI to transfer assets

    let asset_mint_key = vault_state.asset_mint;
    let authority_bump = vault_state.authority_bump;
    let authority_seeds: &[&[u8]] = &[
        VAULT_AUTHORITY_SEED,
        asset_mint_key.as_ref(),
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];

    // Transfer from vault to target
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.target_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, amount)?;

    // Emit event for tracking
    emit!(Invested {
        vault: ctx.accounts.vault_state.key(),
        authority: ctx.accounts.authority.key(),
        target,
        protocol_name,
        amount,
        total_assets: vault_state.total_assets,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}

