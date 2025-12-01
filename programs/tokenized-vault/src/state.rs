use anchor_lang::prelude::*;

/// Global vault state tracking assets and shares
///
/// Security considerations:
/// - Authority stored in state (not instruction args)
/// - Total assets and shares tracked for ERC-4626 math
/// - Bumps stored for efficient PDA signing
/// - 128 bytes padding for future upgrades
#[account]
pub struct VaultState {
    /// Authority that can invest vault assets and manage protocol registry
    pub authority: Pubkey,          // 32 bytes
    
    /// Mint of the underlying asset token
    pub asset_mint: Pubkey,         // 32 bytes
    
    /// Mint of the vault share token
    pub share_mint: Pubkey,         // 32 bytes
    
    /// Total assets held by the vault (including invested)
    pub total_assets: u64,          // 8 bytes
    
    /// Total shares issued to depositors
    pub total_shares: u64,          // 8 bytes
    
    /// Bump seed for vault state PDA
    pub bump: u8,                   // 1 byte
    
    /// Bump seed for share mint PDA
    pub share_bump: u8,             // 1 byte
    
    /// Bump seed for vault authority PDA
    pub authority_bump: u8,         // 1 byte
    
    // Padding for future upgrades
    pub _reserved: [u8; 128],       // 128 bytes
}

/// Protocol registry for approved investment targets
///
/// Architecture: Registry + Whitelist (Option 3)
/// - Single program with upgradeable on-chain whitelist
/// - Authority can add/remove/toggle protocols via instructions
/// - Prevents rug pulls by restricting investment destinations
/// - Tracks invested amount per protocol for transparency
///
/// Security: Authority-controlled whitelist prevents investing to arbitrary addresses
#[account]
pub struct ProtocolRegistry {
    /// Vault this registry belongs to
    pub vault: Pubkey,              // 32 bytes
    
    /// List of approved protocol program IDs
    /// Max ~15 protocols before hitting account size limits
    pub approved_protocols: Vec<ApprovedProtocol>, // 4 + (n * ~80) bytes
    
    /// Bump seed for PDA
    pub bump: u8,                   // 1 byte
}

/// Individual approved protocol entry
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct ApprovedProtocol {
    /// Program ID or token account of the protocol
    pub target: Pubkey,             // 32 bytes
    
    /// Whether this protocol is currently enabled
    pub enabled: bool,              // 1 byte
    
    /// Amount currently invested in this protocol
    pub invested_amount: u64,       // 8 bytes
    
    /// Human-readable name (e.g., "Marinade", "Kamino")
    pub name: String,               // 4 + up to 32 bytes
}

impl ProtocolRegistry {
    /// Space calculation: accommodates up to 10 protocols comfortably
    /// 8 (discriminator) + 32 (vault) + 4 (vec len) + (10 * 80) + 1 (bump) + 128 (padding)
    pub const SPACE: usize = 8 + 32 + 4 + (10 * 80) + 1 + 128;

    /// Check if a protocol target is approved and enabled
    pub fn is_protocol_approved(&self, target: &Pubkey) -> bool {
        self.approved_protocols
            .iter()
            .any(|p| p.target == *target && p.enabled)
    }

    /// Get mutable protocol by target
    pub fn get_protocol_mut(&mut self, target: &Pubkey) -> Option<&mut ApprovedProtocol> {
        self.approved_protocols
            .iter_mut()
            .find(|p| p.target == *target)
    }

    /// Track investment amount for a protocol
    pub fn track_investment(&mut self, target: &Pubkey, amount: u64) -> Result<()> {
        if let Some(protocol) = self.get_protocol_mut(target) {
            protocol.invested_amount = protocol
                .invested_amount
                .checked_add(amount)
                .ok_or(error!(crate::errors::VaultError::MathOverflow))?;
        }
        Ok(())
    }
}

impl VaultState {
    /// Calculate shares to mint for a given asset amount
    ///
    /// ERC-4626 formula:
    /// - If first deposit: shares = assets
    /// - Otherwise: shares = assets * totalShares / totalAssets
    ///
    /// Security: Uses checked math to prevent overflow
    pub fn calculate_shares(&self, assets: u64) -> Result<u64> {
        // First deposit: 1:1 ratio
        if self.total_shares == 0 || self.total_assets == 0 {
            return Ok(assets);
        }

        // Subsequent deposits: proportional to current ratio
        // shares = assets * total_shares / total_assets
        // Using u128 for intermediate calculation to prevent overflow
        let assets_u128 = assets as u128;
        let total_shares_u128 = self.total_shares as u128;
        let total_assets_u128 = self.total_assets as u128;

        let shares_u128 = assets_u128
            .checked_mul(total_shares_u128)
            .ok_or(error!(crate::errors::VaultError::MathOverflow))?
            .checked_div(total_assets_u128)
            .ok_or(error!(crate::errors::VaultError::DivisionByZero))?;

        // Convert back to u64
        u64::try_from(shares_u128)
            .map_err(|_| error!(crate::errors::VaultError::MathOverflow))
    }

    /// Calculate asset value of shares
    ///
    /// ERC-4626 formula: assets = shares * totalAssets / totalShares
    ///
    /// Security: Uses checked math to prevent overflow
    pub fn calculate_assets(&self, shares: u64) -> Result<u64> {
        if self.total_shares == 0 {
            return Ok(0);
        }

        let shares_u128 = shares as u128;
        let total_assets_u128 = self.total_assets as u128;
        let total_shares_u128 = self.total_shares as u128;

        let assets_u128 = shares_u128
            .checked_mul(total_assets_u128)
            .ok_or(error!(crate::errors::VaultError::MathOverflow))?
            .checked_div(total_shares_u128)
            .ok_or(error!(crate::errors::VaultError::DivisionByZero))?;

        u64::try_from(assets_u128)
            .map_err(|_| error!(crate::errors::VaultError::MathOverflow))
    }
}
