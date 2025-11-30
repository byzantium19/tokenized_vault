use anchor_lang::prelude::*;

/// Event emitted when a new vault is initialized
#[event]
pub struct VaultInitialized {
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub asset_mint: Pubkey,
    pub share_mint: Pubkey,
    pub timestamp: i64,
}

/// Event emitted when assets are deposited
#[event]
pub struct Deposited {
    pub vault: Pubkey,
    pub user: Pubkey,
    pub asset_amount: u64,
    pub shares_minted: u64,
    pub total_assets: u64,
    pub total_shares: u64,
    pub timestamp: i64,
}

/// Event emitted when vault assets are invested
#[event]
pub struct Invested {
    pub vault: Pubkey,
    pub authority: Pubkey,
    pub target: Pubkey,
    pub protocol_name: String,
    pub amount: u64,
    pub total_assets: u64,
    pub timestamp: i64,
}

/// Event emitted when a protocol is added to the registry
#[event]
pub struct ProtocolAdded {
    pub vault: Pubkey,
    pub target: Pubkey,
    pub name: String,
    pub timestamp: i64,
}

/// Event emitted when a protocol is toggled
#[event]
pub struct ProtocolToggled {
    pub vault: Pubkey,
    pub target: Pubkey,
    pub enabled: bool,
    pub timestamp: i64,
}

