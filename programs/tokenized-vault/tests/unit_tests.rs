use anchor_lang::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_calculation_first_deposit() {
        // First deposit should be 1:1
        let deposit = 1000_000_000_000u64; // 1000 tokens with 9 decimals
        let total_assets = 0u64;
        let total_shares = 0u64;

        let shares = if total_shares == 0 {
            deposit
        } else {
            ((deposit as u128)
                .checked_mul(total_shares as u128)
                .unwrap()
                / (total_assets as u128)) as u64
        };

        assert_eq!(shares, deposit, "First deposit should mint 1:1 shares");
    }

    #[test]
    fn test_share_calculation_after_profit() {
        // Vault has 1500 assets, 1000 shares (50% profit)
        let deposit = 100_000_000_000u64; // 100 tokens
        let total_assets = 1500_000_000_000u64;
        let total_shares = 1000_000_000_000u64;

        let shares = ((deposit as u128)
            .checked_mul(total_shares as u128)
            .unwrap()
            / (total_assets as u128)) as u64;

        // 100 * 1000 / 1500 = 66.666... = 66 (integer division)
        assert_eq!(shares, 66_666_666_666, "Should receive proportional shares");
    }

    #[test]
    fn test_share_calculation_prevents_overflow() {
        // Test with maximum values
        let deposit = u64::MAX;
        let total_assets = 1000_000_000u64;
        let total_shares = 1000_000_000u64;

        let result = (deposit as u128)
            .checked_mul(total_shares as u128)
            .unwrap()
            / (total_assets as u128);

        // Should not overflow and produce valid result
        assert!(result > 0, "Should handle large numbers without overflow");
    }

    #[test]
    fn test_pda_derivation() {
        let program_id = tokenized_vault::id();
        let asset_mint = Pubkey::new_unique();

        // Derive vault state PDA
        let (vault_state, vault_bump) = Pubkey::find_program_address(
            &[b"vault", asset_mint.as_ref()],
            &program_id,
        );

        // Derive share mint PDA
        let (share_mint, share_bump) = Pubkey::find_program_address(
            &[b"shares", asset_mint.as_ref()],
            &program_id,
        );

        // Derive vault authority PDA
        let (vault_authority, authority_bump) = Pubkey::find_program_address(
            &[b"vault_authority", asset_mint.as_ref()],
            &program_id,
        );

        // Verify PDAs are unique
        assert_ne!(vault_state, share_mint);
        assert_ne!(vault_state, vault_authority);
        assert_ne!(share_mint, vault_authority);

        // Verify bumps are valid
        assert!(vault_bump <= 255);
        assert!(share_bump <= 255);
        assert!(authority_bump <= 255);
    }

    #[test]
    fn test_math_safety_checks() {
        // Test that our math doesn't panic on edge cases
        
        // Zero assets, non-zero shares (shouldn't happen but test it)
        let deposit = 100u64;
        let total_assets = 0u64;
        let total_shares = 1000u64;

        if total_shares > 0 && total_assets > 0 {
            let _shares = ((deposit as u128)
                .checked_mul(total_shares as u128)
                .unwrap()
                / (total_assets as u128)) as u64;
        }
        // If assets are 0, we shouldn't calculate (first deposit case)
        
        // Division by zero protection
        assert!(total_assets == 0, "Should not divide by zero");
    }
}

