use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{constants::*, events::*, state::*};

/// Initialize a new vault for a given asset token
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Vault authority - can invest vault assets
    /// Security: Must be signer, stored in state
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Vault state PDA
    /// Security: Initialized with proper space and padding for upgrades
    #[account(
        init,
        payer = authority,
        space = VAULT_STATE_SIZE,
        seeds = [VAULT_SEED, asset_mint.key().as_ref()],
        bump
    )]
    pub vault_state: Account<'info, VaultState>,

    /// Asset token mint (the underlying token users deposit)
    /// Security: No constraints needed - any valid mint can have a vault
    pub asset_mint: Account<'info, Mint>,

    /// Share token mint PDA (vault shares)
    /// Security: Mint authority is vault_authority PDA
    #[account(
        init,
        payer = authority,
        seeds = [SHARE_MINT_SEED, asset_mint.key().as_ref()],
        bump,
        mint::decimals = asset_mint.decimals,
        mint::authority = vault_authority,
    )]
    pub share_mint: Account<'info, Mint>,

    /// Vault authority PDA - used as mint authority for shares
    /// Security: CHECK constraint ensures correct derivation
    /// CHECK: PDA used as mint authority, validated by seeds
    #[account(
        seeds = [VAULT_AUTHORITY_SEED, asset_mint.key().as_ref()],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// Vault's token account for holding assets
    /// Security: Owned by vault_authority PDA, correct mint
    #[account(
        init,
        payer = authority,
        associated_token::mint = asset_mint,
        associated_token::authority = vault_authority,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let vault_state = &mut ctx.accounts.vault_state;

    // EFFECTS: Initialize vault state
    vault_state.authority = ctx.accounts.authority.key();
    vault_state.asset_mint = ctx.accounts.asset_mint.key();
    vault_state.share_mint = ctx.accounts.share_mint.key();
    vault_state.total_assets = 0;
    vault_state.total_shares = 0;
    vault_state.bump = ctx.bumps.vault_state;
    vault_state.share_bump = ctx.bumps.share_mint;
    vault_state.authority_bump = ctx.bumps.vault_authority;
    vault_state._reserved = [0; 128];

    // INTERACTIONS: Emit event
    emit!(VaultInitialized {
        vault: vault_state.key(),
        authority: vault_state.authority,
        asset_mint: vault_state.asset_mint,
        share_mint: vault_state.share_mint,
        timestamp: Clock::get()?.unix_timestamp,
    });

   // Vault initialized successfully

    Ok(())
}


