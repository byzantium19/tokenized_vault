// Constants for the Tokenized Vault program

/// Seed for vault state PDA
pub const VAULT_SEED: &[u8] = b"vault";

/// Seed for share mint PDA
pub const SHARE_MINT_SEED: &[u8] = b"shares";

/// Seed for vault token account PDA
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";

/// Space for VaultState account (8 discriminator + 32 authority + 32 asset_mint + 
/// 32 share_mint + 8 total_assets + 8 total_shares + 1 bump + 1 share_bump + 
/// 1 authority_bump + 128 padding)
pub const VAULT_STATE_SIZE: usize = 8 + 32 + 32 + 32 + 8 + 8 + 1 + 1 + 1 + 128;


