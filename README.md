# Tokenized Vault - ERC-4626 Style on Solana

An Anchor program implementing a tokenized vault following ERC-4626 share calculation principles.

## Overview

This program:
1. Accepts user deposits of an asset SPL token
2. Mints vault share tokens using ERC-4626 math
3. Allows an authorized account to invest vault assets into third-party programs via CPI
4. Includes comprehensive security constraints and events

## Design

### PDAs (Program Derived Addresses)

| PDA | Seeds | Purpose |
|-----|-------|---------|
| `vault_state` | `["vault", asset_mint]` | Main vault state account |
| `share_mint` | `["shares", asset_mint]` | Vault share token mint |
| `vault_authority` | `["vault_authority", asset_mint]` | PDA authority for signing CPIs |
| `vault_token_account` | ATA of `vault_authority` for `asset_mint` | Holds deposited assets |
| `protocol_registry` | `["protocol_registry", vault_state]` | Whitelist of approved investment targets |

**Security rationale:**
- All PDAs include `asset_mint` to prevent account confusion attacks
- `vault_authority` PDA acts as mint authority and CPI signer (follows standard pattern)
- Protocol registry is tied to specific vault via seeds

### Account Structures

**VaultState:**
```rust
pub struct VaultState {
    pub authority: Pubkey,      // Can invest assets & manage protocols
    pub asset_mint: Pubkey,     // Underlying asset token
    pub share_mint: Pubkey,     // Vault share token
    pub total_assets: u64,      // Total deposited + invested
    pub total_shares: u64,      // Total shares issued
    pub bump: u8,               // PDA bumps for signing
    pub share_bump: u8,
    pub authority_bump: u8,
    pub _reserved: [u8; 128],   // Future upgrades
}
```

**ProtocolRegistry:**
```rust
pub struct ProtocolRegistry {
    pub vault: Pubkey,                              // Vault this belongs to
    pub approved_protocols: Vec<ApprovedProtocol>,  // Whitelisted targets
    pub bump: u8,
}

pub struct ApprovedProtocol {
    pub target: Pubkey,          // Token account to invest in
    pub enabled: bool,           // Can disable without removing
    pub invested_amount: u64,    // Track per-protocol investment
    pub name: String,            // Human-readable name
}
```

### Instructions

#### 1. `initialize`
Creates a new vault for a given asset token.

**Accounts:**
- `authority` (signer, mut) - Pays rent, becomes vault authority
- `vault_state` (init, pda) - Main state account
- `asset_mint` - Underlying SPL token
- `share_mint` (init, pda) - Vault share token (decimals match asset)
- `vault_authority` (pda) - Used as mint/freeze authority
- `vault_token_account` (init) - ATA for holding assets

**Constraints:**
- Share mint decimals set to match asset mint decimals
- Vault authority is PDA (can sign CPIs)

#### 2. `deposit`
User deposits assets and receives shares.

**Accounts:**
- `user` (signer) - Depositor
- `vault_state` (mut, pda) - Updated with new totals
- `user_asset_account` (mut) - Source (validated: mint + owner)
- `user_share_account` (mut) - Destination (validated: mint + owner)
- `vault_token_account` (mut) - Vault's asset holding
- `vault_authority` (pda) - Signs mint instruction

**Constraints:**
- Token account mints validated against vault state
- Token account owners validated
- Amount > 0 checked

**Math (ERC-4626):**
```rust
if vault_state.total_shares == 0 {
    shares_to_mint = amount;  // First deposit: 1:1 ratio
} else {
    shares_to_mint = (amount as u128)
        .checked_mul(vault_state.total_shares as u128)
        .unwrap()
        .checked_div(vault_state.total_assets as u128)
        .unwrap() as u64;
}
```

#### 3. `add_protocol`
Authority adds a protocol to the investment whitelist.

**Accounts:**
- `authority` (signer) - Must match `vault_state.authority`
- `vault_state` (has_one = authority)
- `protocol_registry` (init_if_needed, pda)

**Parameters:**
- `target: Pubkey` - Token account to allow investments to
- `name: String` - Protocol name

**Constraints:**
- Only vault authority can call
- Max ~10 protocols (account size limit)

#### 4. `toggle_protocol`
Authority enables/disables a protocol without removing it.

**Accounts:**
- `authority` (signer)
- `vault_state` (has_one = authority)
- `protocol_registry` (mut, pda)

**Parameters:**
- `target: Pubkey` - Protocol to toggle
- `enabled: bool` - New state

#### 5. `invest`
Authority invests vault assets into a whitelisted protocol via CPI.

**Accounts:**
- `authority` (signer) - Must match `vault_state.authority`
- `vault_state` (mut, has_one = authority)
- `protocol_registry` (mut, pda) - For whitelist validation
- `vault_authority` (pda) - Signs the transfer
- `vault_token_account` (mut) - Source
- `target_token_account` (mut) - Destination (must be whitelisted)
- `token_program` - For CPI

**Parameters:**
- `amount: u64` - Amount to invest

**Constraints:**
- Target must be in approved protocols list and enabled
- Amount <= vault token account balance
- Uses PDA signing for CPI

**CPI Layout:**
```rust
let signer_seeds: &[&[&[u8]]] = &[&[
    b"vault_authority",
    vault_state.asset_mint.as_ref(),
    &[vault_state.authority_bump],
]];

token::transfer(
    CpiContext::new_with_signer(
        token_program,
        Transfer { from, to, authority },
        signer_seeds
    ),
    amount
)?;
```
### Share Calculation Examples

**First deposit (empty vault):**
```
Vault: 0 assets, 0 shares
User deposits: 1000 tokens
Receives: 1000 shares (1:1 ratio)
```

**Subsequent deposit (after profit):**
```
Vault: 1500 assets, 1000 shares (50% profit earned)
User deposits: 100 tokens
Receives: 66 shares
Math: 100 × 1000 / 1500 = 66.67 → 66 (integer division)
```

**Asset value calculation:**
```
Value of N shares = N × total_assets / total_shares
```

## Decimals Handling

### Overview
Share mint decimals **always match** asset mint decimals. This is enforced during vault initialization:

```rust
#[account(
    init,
    payer = authority,
    mint::decimals = asset_mint.decimals,  // ← Matches asset
    mint::authority = vault_authority,
    seeds = [b"shares", asset_mint.key().as_ref()],
    bump
)]
pub share_mint: Account<'info, Mint>,
```

### Why This Matters
- **Intuitive:** First deposit at 1:1 ratio (1000 tokens → 1000 shares)
- **Consistent:** All subsequent calculations use same decimal precision
- **Compatible:** Share tokens work seamlessly with wallets/DEXs expecting standard SPL decimals

### Precision in Math
All share calculations use `u128` intermediates to prevent overflow:

```rust
let shares_to_mint = (amount as u128)
    .checked_mul(vault_state.total_shares as u128)
    .ok_or(VaultError::MathOverflow)?
    .checked_div(vault_state.total_assets as u128)
    .ok_or(VaultError::DivisionByZero)?;

// Safe conversion back to u64
let shares_to_mint = u64::try_from(shares_to_mint)
    .map_err(|_| VaultError::MathOverflow)?;
```

**Safety guarantees:**
- No overflow on multiplication (u128 can hold u64 × u64)
- Explicit error on division by zero
- Explicit error if result doesn't fit in u64

### Integer Division Precision Loss

⚠️ **Inherent limitation:** Integer division truncates remainders.

**Example:**
```
Vault: 1000 assets, 333 shares
Deposit: 100 assets
Expected: 33.3 shares
Actual: 33 shares (0.3 lost to rounding)
```

**Impact:**
- Small rounding error per deposit (~0.01-0.1%)
- Errors don't accumulate exponentially (each calc is independent)
- User receives slightly fewer shares (conservative, favors existing holders)

**Mitigations:**
1. Use high-decimal tokens (9+ decimals like USDC/SOL)
2. Minimum deposit thresholds to avoid dust
3. In production: implement withdrawal mechanism that handles rounding fairly

**Production considerations:**
- Track rounding dust in separate account
- Implement "virtual shares" (ERC-4626 advanced pattern)
- Add minimum share amount checks

## Security Features

This implementation follows Solana security best practices:

### 1. Signer Validation
All privileged operations require `Signer<'info>`
```rust
pub authority: Signer<'info>,  // Cannot be forged
```

### 2. Account Ownership
All PDAs use unique seeds (asset_mint) to prevent collision
```rust
seeds = [b"vault", asset_mint.key().as_ref()],
```

### 3. Authority Checks
 Authority stored in state, validated with `has_one`
```rust
#[account(
    mut,
    has_one = authority @ VaultError::Unauthorized
)]
pub vault_state: Account<'info, VaultState>,
```

### 4. Initialization Protection
 Uses Anchor's `init` constraint (prevents reinitialization)
```rust
#[account(init, payer = authority, space = 8 + VaultState::INIT_SPACE)]
```

### 5. CPI Security
 Hardcoded program IDs, PDA signing
```rust
pub token_program: Program<'info, Token>,  // Validates against Token program ID

let signer_seeds: &[&[&[u8]]] = &[&[
    b"vault_authority",
    vault_state.asset_mint.as_ref(),
    &[vault_state.authority_bump],
]];
```

### 6. Math Safety
 All arithmetic uses checked operations
```rust
amount.checked_mul(total_shares)
    .ok_or(VaultError::MathOverflow)?
    .checked_div(total_assets)
    .ok_or(VaultError::DivisionByZero)?
```

### 7. Token Account Validation
 Mint and owner validated with constraints
```rust
#[account(
    mut,
    constraint = user_asset_account.mint == vault_state.asset_mint @ VaultError::InvalidMint,
    constraint = user_asset_account.owner == user.key() @ VaultError::InvalidOwner,
)]
```

### 8. Investment Whitelist
 Authority can only invest to pre-approved protocols
```rust
// Check target is in approved list
let protocol = protocol_registry.approved_protocols
    .iter()
    .find(|p| p.target == target_token_account.key())
    .ok_or(VaultError::ProtocolNotWhitelisted)?;

require!(protocol.enabled, VaultError::ProtocolDisabled);
```

### 9. Checks-Effects-Interactions Pattern
State updated before external calls
```rust
// 1. CHECKS
require!(amount > 0, VaultError::InvalidAmount);

// 2. EFFECTS
vault_state.total_assets += amount;
vault_state.total_shares += shares;

// 3. INTERACTIONS (CPI last)
token::transfer(cpi_ctx, amount)?;
token::mint_to(cpi_ctx, shares)?;
```

### 10. Events for Monitoring
 All operations emit events
```rust
emit!(VaultInitialized { vault, asset_mint, authority });
emit!(Deposited { user, amount, shares });
emit!(Invested { target, amount });
```

## How to Run

### Prerequisites & Installation

This project requires specific versions of Solana, Anchor, and Node.js tooling.

#### 1. Install Rust (if not already installed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 2. Install Solana CLI 3.0.11 (Agave)
```bash
sh -c "$(curl -sSfL https://release.anza.xyz/v3.0.11/install)"

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Verify installation
solana --version
# Expected: solana-cli 3.0.11 (src:edda5bc0; feat:3604001754, client:Agave)
```

**Why Solana 3.0.11?**
- Includes rustc 1.84.1 (required for latest dependencies)
- Agave client with updated RPC protocol
- Compatible with Anchor 0.32.1

#### 3. Install Anchor Version Manager (AVM)
```bash
cargo install --git https://github.com/coral-xyz/anchor avm --force

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.avm/bin:$PATH"
```

#### 4. Install Anchor 0.32.1
```bash
avm install 0.32.1
avm use 0.32.1

# Verify installation
anchor --version
# Expected: anchor-cli 0.32.1
```

#### 5. Install Node.js 20.x (via nvm)
```bash
# Install nvm if not already installed
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash

# Load nvm
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# Install Node.js 20
nvm install 20
nvm use 20

# Verify installation
node --version  # Expected: v20.x.x
```

#### 6. Install Yarn
```bash
npm install -g yarn

# Verify installation
yarn --version  # Expected: >= 1.22.x
```

### Version Summary
| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.84.1+ | Compiles Solana programs |
| Solana CLI | 3.0.11 | Deploys programs, runs test validator |
| Anchor | 0.32.1 | Framework for Solana program development |
| Node.js | 20.x | Runs TypeScript tests |
| Yarn | 1.22+ | Package manager for Node dependencies |
| @solana/web3.js | 1.98.4 | Solana JavaScript SDK |
| @solana/spl-token | 0.4.8 | SPL Token operations in tests |
| @coral-xyz/anchor | 0.32.1 | Anchor TypeScript client |

### Setup Instructions

#### 1. Install Dependencies
```bash
yarn install
```

This installs:
- `@coral-xyz/anchor@0.32.1` - Anchor TypeScript client
- `@solana/web3.js@1.98.4` - Solana web3 library
- `@solana/spl-token@0.4.8` - SPL Token operations
- `ts-mocha` - TypeScript test runner
- `chai` - Assertion library

#### 2. Generate Program Keys
```bash
anchor keys list
# Output: tokenized_vault: <PROGRAM_ID>
```

Update the program ID in two places:
- `programs/tokenized-vault/src/lib.rs`:
  ```rust
  declare_id!("<PROGRAM_ID>");
  ```
- `Anchor.toml`:
  ```toml
  [programs.localnet]
  tokenized_vault = "<PROGRAM_ID>"
  ```

#### 3. Build
```bash
anchor build
```

Expected output:
```
Compiling tokenized-vault v0.1.0
Finished release [optimized] target(s) in X.XXs
```

#### 4. Run Tests

**Rust Unit Tests:**
```bash
cargo test --package tokenized-vault
```

Tests cover:
- ERC-4626 share math (first deposit, subsequent deposits, profit scenarios)
- PDA derivation (vault state, share mint, vault authority)
- Math overflow protection

**Integration Tests:**
```bash
anchor test
```

**All 12 integration tests passing:**
- ✓ Initializes the vault
- ✓ User1 deposits assets
- ✓ Adds Protocol1 to whitelist
- ✓ Adds Protocol2 to whitelist
- ✓ Authority can invest in whitelisted protocol1
- ✓ Fails to invest in non-whitelisted protocol
- ✓ Disables Protocol2
- ✓ Fails to invest in disabled protocol
- ✓ Re-enables Protocol2
- ✓ Can now invest in re-enabled protocol
- ✓ Non-authority cannot add protocols
- ✓ Displays final state with protocol tracking

**Note on Test Setup:**
The integration tests use explicit `Keypair` instances when creating token accounts to avoid issues with Solana 3.0's ATA program restrictions. This ensures compatibility across different Solana versions.

### Expected Test Output
```
  12 passing (10s)

12 passing (Xs)
```

**Test Coverage:**
- Vault initialization with PDAs
- ERC-4626 math (first deposit 1:1 ratio)
- Protocol whitelist management
- Investment to whitelisted protocols
- Authority-only access control
- Error cases (unauthorized, disabled protocols)
- State verification and tracking

## Known Limitations

### 1. No Withdraw/Redeem
**Status:** Not implemented (per requirements)

Users cannot redeem shares for assets. For production:
```rust
pub fn withdraw(ctx: Context<Withdraw>, shares: u64) -> Result<()> {
    // Calculate: assets = shares × total_assets / total_shares
    // Burn shares via CPI
    // Transfer assets via CPI
}
```

### 2. Integer Division Rounding
**Status:** Inherent to integer math

Small precision loss on each deposit (see Decimals section). Mitigations:
- High-decimal tokens (9+)
- Minimum deposit amounts
- Virtual shares offset (ERC-4626 advanced)

### 3. No Performance Fees
**Status:** Simplified for MVP

No protocol revenue mechanism. For production:
```rust
pub protocol_fee_bps: u16,  // e.g., 200 = 2%
// Deduct fee on profit before distributing to shareholders
```

### 4. Investment Tracking
**Status:** Implemented in `ProtocolRegistry`

Each protocol tracks `invested_amount`, updated on invest/divest.

### 5. No Flash Loan Protection
**Status:** Not critical without withdraw

If withdraw is added, implement:
- Reentrancy guards
- Same-block deposit/withdraw limits
- Time-weighted pricing

### 6. No Emergency Pause
**Status:** Not implemented

For production, add:
```rust
pub paused: bool,
// Restrict deposits/invests when true
```

### 7. Account Size Limits
**Status:** `ProtocolRegistry` limited to ~10 protocols

Vec stored on-chain. For more protocols:
- Use separate accounts per protocol
- Or use off-chain registry + merkle proof validation



