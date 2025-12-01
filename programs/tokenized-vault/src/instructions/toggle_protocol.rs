use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, events::*, state::*};

/// Toggle a protocol on/off in the whitelist
#[derive(Accounts)]
pub struct ToggleProtocol<'info> {
    /// Vault authority - only they can manage protocols
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Vault state PDA
    #[account(
        seeds = [VAULT_SEED, vault_state.asset_mint.as_ref()],
        bump = vault_state.bump,
        has_one = authority @ VaultError::Unauthorized,
    )]
    pub vault_state: Account<'info, VaultState>,

    /// Protocol registry PDA
    #[account(
        mut,
        seeds = [b"protocol_registry", vault_state.key().as_ref()],
        bump = protocol_registry.bump,
    )]
    pub protocol_registry: Account<'info, ProtocolRegistry>,
}

pub fn handler(
    ctx: Context<ToggleProtocol>,
    target: Pubkey,
    enabled: bool,
) -> Result<()> {
    let registry = &mut ctx.accounts.protocol_registry;

    // Find and toggle protocol
    let protocol = registry
        .get_protocol_mut(&target)
        .ok_or(VaultError::ProtocolNotFound)?;

    protocol.enabled = enabled;

    // Emit event
    emit!(ProtocolToggled {
        vault: registry.vault,
        target,
        enabled,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}


