import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TokenizedVault } from "../target/types/tokenized_vault";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccount,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert } from "chai";

describe("tokenized-vault with Protocol Registry", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenizedVault as Program<TokenizedVault>;

  let assetMint: anchor.web3.PublicKey;
  let authority: anchor.web3.Keypair;
  let user1: anchor.web3.Keypair;
  let user2: anchor.web3.Keypair;

  // PDAs
  let vaultState: anchor.web3.PublicKey;
  let shareMint: anchor.web3.PublicKey;
  let vaultAuthority: anchor.web3.PublicKey;
  let vaultTokenAccount: anchor.web3.PublicKey;
  let protocolRegistry: anchor.web3.PublicKey;

  // User token accounts
  let user1AssetAccount: anchor.web3.PublicKey;
  let user1ShareAccount: anchor.web3.PublicKey;
  let user2AssetAccount: anchor.web3.PublicKey;
  let user2ShareAccount: anchor.web3.PublicKey;

  // Investment targets
  let protocol1Target: anchor.web3.PublicKey;
  let protocol2Target: anchor.web3.PublicKey;
  let unauthorizedTarget: anchor.web3.PublicKey;

  before(async () => {
    // Create test keypairs
    authority = anchor.web3.Keypair.generate();
    user1 = anchor.web3.Keypair.generate();
    user2 = anchor.web3.Keypair.generate();

    // Airdrop SOL and wait for confirmation
    const airdrop1 = await provider.connection.requestAirdrop(
      authority.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdrop1);
    
    const airdrop2 = await provider.connection.requestAirdrop(
      user1.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdrop2);
    
    const airdrop3 = await provider.connection.requestAirdrop(
      user2.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdrop3);

    // Create asset mint (9 decimals)
    assetMint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      9
    );

    // Create user asset accounts using getOrCreateAssociatedTokenAccount
    // Authority pays for ATAs since it's the mint authority
    const user1AssetAccountInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      authority,
      assetMint,
      user1.publicKey,
      false
    );
    user1AssetAccount = user1AssetAccountInfo.address;
    
    const user2AssetAccountInfo = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      authority,
      assetMint,
      user2.publicKey,
      false
    );
    user2AssetAccount = user2AssetAccountInfo.address;

    // Mint tokens to users
    await mintTo(
      provider.connection,
      authority,
      assetMint,
      user1AssetAccount,
      authority,
      1000 * 1e9
    );

    await mintTo(
      provider.connection,
      authority,
      assetMint,
      user2AssetAccount,
      authority,
      1000 * 1e9
    );

    // Create investment target accounts (must provide keypairs to create regular token accounts)
    const protocol1Keypair = anchor.web3.Keypair.generate();
    protocol1Target = await createAccount(
      provider.connection,
      authority,
      assetMint,
      authority.publicKey,
      protocol1Keypair
    );

    const protocol2Keypair = anchor.web3.Keypair.generate();
    protocol2Target = await createAccount(
      provider.connection,
      authority,
      assetMint,
      authority.publicKey,
      protocol2Keypair
    );

    const unauthorizedKeypair = anchor.web3.Keypair.generate();
    unauthorizedTarget = await createAccount(
      provider.connection,
      authority,
      assetMint,
      authority.publicKey,
      unauthorizedKeypair
    );

    // Derive PDAs
    [vaultState] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), assetMint.toBuffer()],
      program.programId
    );

    [shareMint] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("shares"), assetMint.toBuffer()],
      program.programId
    );

    [vaultAuthority] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_authority"), assetMint.toBuffer()],
      program.programId
    );

    [protocolRegistry] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("protocol_registry"), vaultState.toBuffer()],
      program.programId
    );

    vaultTokenAccount = await getAssociatedTokenAddress(
      assetMint,
      vaultAuthority,
      true
    );
  });

  it("Initializes the vault", async () => {
    const tx = await program.methods
      .initialize()
      .accounts({
        authority: authority.publicKey,
        vaultState,
        assetMint,
        shareMint,
        vaultAuthority,
        vaultTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    console.log("✓ Vault initialized");

    const vaultStateAccount = await program.account.vaultState.fetch(vaultState);
    assert.equal(vaultStateAccount.authority.toString(), authority.publicKey.toString());
    assert.equal(vaultStateAccount.totalAssets.toNumber(), 0);
  });

  it("User1 deposits assets", async () => {
    user1ShareAccount = await createAssociatedTokenAccount(
      provider.connection,
      user1,
      shareMint,
      user1.publicKey
    );

    const depositAmount = new anchor.BN(100 * 1e9);

    await program.methods
      .deposit(depositAmount)
      .accounts({
        user: user1.publicKey,
        vaultState,
        assetMint,
        shareMint,
        vaultAuthority,
        userAssetAccount: user1AssetAccount,
        userShareAccount: user1ShareAccount,
        vaultTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user1])
      .rpc();

    console.log("✓ User1 deposited 100 tokens");

    const userShares = await getAccount(provider.connection, user1ShareAccount);
    assert.equal(userShares.amount.toString(), depositAmount.toString());
  });

  it("Adds Protocol1 to whitelist", async () => {
    await program.methods
      .addProtocol(protocol1Target, "Marinade")
      .accounts({
        authority: authority.publicKey,
        vaultState,
        protocolRegistry,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    console.log("✓ Protocol1 (Marinade) added to whitelist");

    const registry = await program.account.protocolRegistry.fetch(protocolRegistry);
    assert.equal(registry.approvedProtocols.length, 1);
    assert.equal(registry.approvedProtocols[0].name, "Marinade");
    assert.equal(registry.approvedProtocols[0].enabled, true);
  });

  it("Adds Protocol2 to whitelist", async () => {
    await program.methods
      .addProtocol(protocol2Target, "Kamino")
      .accounts({
        authority: authority.publicKey,
        vaultState,
        protocolRegistry,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([authority])
      .rpc();

    console.log("✓ Protocol2 (Kamino) added to whitelist");

    const registry = await program.account.protocolRegistry.fetch(protocolRegistry);
    assert.equal(registry.approvedProtocols.length, 2);
  });

  it("Authority can invest in whitelisted protocol1", async () => {
    const investAmount = new anchor.BN(30 * 1e9);

    await program.methods
      .invest(investAmount)
      .accounts({
        authority: authority.publicKey,
        vaultState,
        protocolRegistry,
        vaultAuthority,
        vaultTokenAccount,
        targetTokenAccount: protocol1Target,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([authority])
      .rpc();

    console.log("✓ Invested 30 tokens in Protocol1");

    const registry = await program.account.protocolRegistry.fetch(protocolRegistry);
    const protocol1 = registry.approvedProtocols.find(
      (p) => p.target.toString() === protocol1Target.toString()
    );
    assert.equal(protocol1.investedAmount.toNumber(), 30 * 1e9);
  });

  it("Fails to invest in non-whitelisted protocol", async () => {
    const investAmount = new anchor.BN(10 * 1e9);

    try {
      await program.methods
        .invest(investAmount)
        .accounts({
          authority: authority.publicKey,
          vaultState,
          protocolRegistry,
          vaultAuthority,
          vaultTokenAccount,
          targetTokenAccount: unauthorizedTarget,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([authority])
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (err) {
      assert.include(err.message, "ProtocolNotApproved");
      console.log("✓ Correctly rejected unauthorized protocol");
    }
  });

  it("Disables Protocol2", async () => {
    await program.methods
      .toggleProtocol(protocol2Target, false)
      .accounts({
        authority: authority.publicKey,
        vaultState,
        protocolRegistry,
      })
      .signers([authority])
      .rpc();

    console.log("✓ Protocol2 disabled");

    const registry = await program.account.protocolRegistry.fetch(protocolRegistry);
    const protocol2 = registry.approvedProtocols.find(
      (p) => p.target.toString() === protocol2Target.toString()
    );
    assert.equal(protocol2.enabled, false);
  });

  it("Fails to invest in disabled protocol", async () => {
    const investAmount = new anchor.BN(10 * 1e9);

    try {
      await program.methods
        .invest(investAmount)
        .accounts({
          authority: authority.publicKey,
          vaultState,
          protocolRegistry,
          vaultAuthority,
          vaultTokenAccount,
          targetTokenAccount: protocol2Target,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([authority])
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (err) {
      assert.include(err.message, "ProtocolNotApproved");
      console.log("✓ Correctly rejected disabled protocol");
    }
  });

  it("Re-enables Protocol2", async () => {
    await program.methods
      .toggleProtocol(protocol2Target, true)
      .accounts({
        authority: authority.publicKey,
        vaultState,
        protocolRegistry,
      })
      .signers([authority])
      .rpc();

    console.log("✓ Protocol2 re-enabled");

    const registry = await program.account.protocolRegistry.fetch(protocolRegistry);
    const protocol2 = registry.approvedProtocols.find(
      (p) => p.target.toString() === protocol2Target.toString()
    );
    assert.equal(protocol2.enabled, true);
  });

  it("Can now invest in re-enabled protocol", async () => {
    const investAmount = new anchor.BN(20 * 1e9);

    await program.methods
      .invest(investAmount)
      .accounts({
        authority: authority.publicKey,
        vaultState,
        protocolRegistry,
        vaultAuthority,
        vaultTokenAccount,
        targetTokenAccount: protocol2Target,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([authority])
      .rpc();

    console.log("✓ Invested 20 tokens in Protocol2 (after re-enable)");

    const registry = await program.account.protocolRegistry.fetch(protocolRegistry);
    const protocol2 = registry.approvedProtocols.find(
      (p) => p.target.toString() === protocol2Target.toString()
    );
    assert.equal(protocol2.investedAmount.toNumber(), 20 * 1e9);
  });

  it("Non-authority cannot add protocols", async () => {
    const fakeProtocolKeypair = anchor.web3.Keypair.generate();
    const fakeProtocol = await createAccount(
      provider.connection,
      user1,
      assetMint,
      user1.publicKey,
      fakeProtocolKeypair  // Provide explicit keypair to avoid ATA creation issue
    );

    try {
      await program.methods
        .addProtocol(fakeProtocol, "Scam Protocol")
        .accounts({
          authority: user1.publicKey,
          vaultState,
          protocolRegistry,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (err) {
      assert.include(err.message, "Unauthorized");
      console.log("✓ Correctly rejected non-authority protocol addition");
    }
  });

  it("Displays final state with protocol tracking", async () => {
    const vaultStateAccount = await program.account.vaultState.fetch(vaultState);
    const registry = await program.account.protocolRegistry.fetch(protocolRegistry);

    console.log("\n=== Final Vault State ===");
    console.log("Total Assets:", vaultStateAccount.totalAssets.toString());
    console.log("Total Shares:", vaultStateAccount.totalShares.toString());

    console.log("\n=== Protocol Registry ===");
    console.log(`Total Protocols: ${registry.approvedProtocols.length}`);
    
    registry.approvedProtocols.forEach((protocol, idx) => {
      console.log(`\nProtocol ${idx + 1}:`);
      console.log(`  Name: ${protocol.name}`);
      console.log(`  Target: ${protocol.target.toString()}`);
      console.log(`  Enabled: ${protocol.enabled}`);
      console.log(`  Invested: ${protocol.investedAmount.toString()}`);
    });

    const totalInvested = registry.approvedProtocols.reduce(
      (sum, p) => sum + Number(p.investedAmount),
      0
    );
    console.log(`\nTotal Invested: ${totalInvested}`);
    console.log(`Remaining in Vault: ${Number(vaultStateAccount.totalAssets) - totalInvested}`);
  });
});
