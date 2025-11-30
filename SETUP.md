# Quick Setup Guide

## Prerequisites Installation

This guide provides step-by-step instructions to set up the development environment from scratch.

### 1. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version  # Should show 1.84+
```

### 2. Install Solana CLI 3.0.11
```bash
sh -c "$(curl -sSfL https://release.anza.xyz/v3.0.11/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
solana --version
# Expected: solana-cli 3.0.11 (src:edda5bc0; feat:3604001754, client:Agave)
```

Add to your `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
```

### 3. Install Anchor Version Manager (AVM)
```bash
cargo install --git https://github.com/coral-xyz/anchor avm --force
export PATH="$HOME/.avm/bin:$PATH"
```

Add to your `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$HOME/.avm/bin:$PATH"
```

### 4. Install Anchor 0.32.1
```bash
avm install 0.32.1
avm use 0.32.1
anchor --version  # Should show: anchor-cli 0.32.1
```

### 5. Install Node.js 20.x via nvm
```bash
# Install nvm
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash

# Load nvm
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# Install Node 20
nvm install 20
nvm use 20
node --version  # Should show v20.x.x
```

### 6. Install Yarn
```bash
npm install -g yarn
yarn --version  # Should show >= 1.22.x
```

## Project Setup

### 1. Clone & Install Dependencies
```bash
cd tokenized-vault
yarn install
```

This will install:
- `@coral-xyz/anchor@0.32.1`
- `@solana/web3.js@1.98.4`
- `@solana/spl-token@0.4.8`
- TypeScript and testing dependencies

### 2. Build the Program
```bash
anchor build
```

### 3. Run Tests

**Rust Unit Tests:**
```bash
cargo test --package tokenized-vault
```

**Integration Tests:**
```bash
anchor test
```

## Verification Checklist

- [ ] `rustc --version` shows 1.84+
- [ ] `solana --version` shows 3.0.11
- [ ] `anchor --version` shows 0.32.1
- [ ] `node --version` shows v20.x.x
- [ ] `yarn --version` shows >= 1.22.x
- [ ] `anchor build` completes successfully
- [ ] `cargo test` shows all tests passing
- [ ] `anchor test` shows all 12 integration tests passing

## Troubleshooting

### Issue: "rustc 1.79 is not supported"
**Solution:** Update Solana to 3.0.11 which includes rustc 1.84.1

### Issue: "anchor: command not found"
**Solution:** Install AVM and Anchor, ensure PATH is set correctly

### Issue: "Connection refused" during anchor test
**Solution:** 
- Kill any existing validators: `pkill -9 solana-test-validator`
- Clean test artifacts: `rm -rf .anchor test-ledger`
- Run `anchor test` again

### Issue: Lock file conflicts
**Solution:** 
- Delete `Cargo.lock` and `yarn.lock`
- Regenerate with `anchor build` and `yarn install`

## File Structure

```
tokenized-vault/
├── Anchor.toml           # Anchor project configuration
├── Cargo.toml            # Rust workspace configuration
├── package.json          # Node.js dependencies
├── tsconfig.json         # TypeScript configuration
├── README.md             # Full documentation
├── SETUP.md              # This file
├── .gitignore            # Git ignore patterns
├── programs/
│   └── tokenized-vault/
│       ├── Cargo.toml    # Program Cargo config
│       ├── src/          # Program source code
│       └── tests/        # Rust unit tests
└── tests/
    └── tokenized-vault.spec.ts  # Integration tests
```

## Next Steps

After successful setup:
1. Read `README.md` for full documentation
2. Review the program source in `programs/tokenized-vault/src/`
3. Examine tests in `tests/` and `programs/tokenized-vault/tests/`
4. Generate new program keys if deploying: `anchor keys list`
