use anchor_lang::prelude::*;

use crate::{constants::*, errors::*, events::*, state::*};

/// Add a new protocol to the approved whitelist
///
/// Security checklist:
/// ✅ 1. SIGNER VALIDATION: Authority must be signer
/// ✅ 3. AUTHORITY CHECKS: has_one constraint validates authority
/// ✅ 4. INITIALIZATION: init constraint for first-time registry creation
/// ✅ 8. BUSINESS LOGIC: Validates name length, checks for duplicates
/// ✅ 9. ACCESS CONTROL: Authority-only function
/// ✅ 10. EVENTS: Emits ProtocolAdded event
#[derive(Accounts)]
pub struct AddProtocol<'info> {
    /// Vault authority - only they can manage protocols
    /// Security: Must be signer and match vault_state.authority
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Vault state PDA
    /// Security: has_one constraint validates authority from state
    #[account(
        seeds = [VAULT_SEED, vault_state.asset_mint.as_ref()],
        bump = vault_state.bump,
        has_one = authority @ VaultError::Unauthorized,
    )]
    pub vault_state: Account<'info, VaultState>,

    /// Protocol registry PDA
    /// Security: Initialized on first add_protocol call
    #[account(
        init_if_needed,
        payer = authority,
        space = ProtocolRegistry::SPACE,
        seeds = [b"protocol_registry", vault_state.key().as_ref()],
        bump
    )]
    pub protocol_registry: Account<'info, ProtocolRegistry>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<AddProtocol>,
    target: Pubkey,
    name: String,
) -> Result<()> {
    // CHECKS: Validate inputs
    require!(name.len() <= 32, VaultError::NameTooLong);

    let registry = &mut ctx.accounts.protocol_registry;

    // Initialize registry if first time
    if registry.vault == Pubkey::default() {
        registry.vault = ctx.accounts.vault_state.key();
        registry.bump = ctx.bumps.protocol_registry;
        registry.approved_protocols = Vec::new();
    }

    // Check if protocol already exists
    require!(
        !registry.approved_protocols.iter().any(|p| p.target == target),
        VaultError::ProtocolAlreadyExists
    );

    // Check registry capacity (max 10 protocols)
    require!(
        registry.approved_protocols.len() < 10,
        VaultError::RegistryFull
    );

    // EFFECTS: Add protocol to registry
    registry.approved_protocols.push(ApprovedProtocol {
        target,
        enabled: true,
        invested_amount: 0,
        name: name.clone(),
    });

    // INTERACTIONS: Emit event
    emit!(ProtocolAdded {
        vault: registry.vault,
        target,
        name,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Protocol added successfully");
    msg!("Target: {}", target);
    msg!("Total protocols: {}", registry.approved_protocols.len());

    Ok(())
}


