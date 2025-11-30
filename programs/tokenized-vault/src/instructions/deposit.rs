use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::{constants::*, errors::*, events::*, state::*};

/// Deposit assets into the vault and receive shares
///
/// Security checklist:
/// ✅ 1. SIGNER VALIDATION: User must be signer
/// ✅ 2. ACCOUNT OWNERSHIP: Vault state PDA validated with seeds
/// ✅ 6. MATH SAFETY: Uses checked operations for share calculation
/// ✅ 7. TOKEN ACCOUNT VALIDATION: Validates mint and owner
/// ✅ 8. BUSINESS LOGIC: Checks-effects-interactions pattern
/// ✅ 10. EVENTS: Emits Deposited event
#[derive(Accounts)]
pub struct Deposit<'info> {
    /// User depositing assets
    /// Security: Must be signer
    #[account(mut)]
    pub user: Signer<'info>,

    /// Vault state PDA
    /// Security: Validated by seeds, contains authority and totals
    #[account(
        mut,
        seeds = [VAULT_SEED, vault_state.asset_mint.as_ref()],
        bump = vault_state.bump,
    )]
    pub vault_state: Account<'info, VaultState>,

    /// Asset mint
    /// Security: Must match vault_state.asset_mint
    #[account(
        address = vault_state.asset_mint,
    )]
    pub asset_mint: Account<'info, Mint>,

    /// Share mint
    /// Security: Must match vault_state.share_mint
    #[account(
        mut,
        address = vault_state.share_mint,
    )]
    pub share_mint: Account<'info, Mint>,

    /// Vault authority PDA
    /// Security: CHECK constraint, validated by seeds
    /// CHECK: PDA used as authority, validated by seeds
    #[account(
        seeds = [VAULT_AUTHORITY_SEED, vault_state.asset_mint.as_ref()],
        bump = vault_state.authority_bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// User's asset token account (source)
    /// Security: Must be owned by user and correct mint
    #[account(
        mut,
        constraint = user_asset_account.mint == vault_state.asset_mint @ VaultError::InvalidMint,
        constraint = user_asset_account.owner == user.key() @ VaultError::InvalidOwner,
    )]
    pub user_asset_account: Account<'info, TokenAccount>,

    /// User's share token account (destination)
    /// Security: Must be owned by user and correct mint
    #[account(
        mut,
        constraint = user_share_account.mint == vault_state.share_mint @ VaultError::InvalidMint,
        constraint = user_share_account.owner == user.key() @ VaultError::InvalidOwner,
    )]
    pub user_share_account: Account<'info, TokenAccount>,

    /// Vault's token account
    /// Security: Must be correct mint and owned by vault_authority
    #[account(
        mut,
        constraint = vault_token_account.mint == vault_state.asset_mint @ VaultError::InvalidMint,
        constraint = vault_token_account.owner == vault_authority.key() @ VaultError::InvalidOwner,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    // CHECKS: Validate amount
    require!(amount > 0, VaultError::ZeroDepositAmount);

    let vault_state = &mut ctx.accounts.vault_state;

    // Calculate shares to mint using ERC-4626 formula
    let shares_to_mint = vault_state.calculate_shares(amount)?;

    // EFFECTS: Update vault state BEFORE external calls
    vault_state.total_assets = vault_state
        .total_assets
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;

    vault_state.total_shares = vault_state
        .total_shares
        .checked_add(shares_to_mint)
        .ok_or(VaultError::MathOverflow)?;

    // INTERACTIONS: External calls after state updates

    // Transfer assets from user to vault
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_asset_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, amount)?;

    // Mint shares to user
    let asset_mint_key = vault_state.asset_mint;
    let authority_bump = vault_state.authority_bump;
    let authority_seeds: &[&[u8]] = &[
        VAULT_AUTHORITY_SEED,
        asset_mint_key.as_ref(),
        &[authority_bump],
    ];
    let signer_seeds = &[&authority_seeds[..]];

    let mint_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            mint: ctx.accounts.share_mint.to_account_info(),
            to: ctx.accounts.user_share_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::mint_to(mint_ctx, shares_to_mint)?;

    // Emit event
    emit!(Deposited {
        vault: vault_state.key(),
        user: ctx.accounts.user.key(),
        asset_amount: amount,
        shares_minted: shares_to_mint,
        total_assets: vault_state.total_assets,
        total_shares: vault_state.total_shares,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}


