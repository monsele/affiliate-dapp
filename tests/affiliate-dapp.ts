import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import {
  createInitializeMintInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  TOKEN_2022_PROGRAM_ID,
  getAccount,
} from "@solana/spl-token";
import { assert } from "chai";
import { AffiliateDapp } from "../target/types/affiliate_dapp"; // Replace with your program type

describe("nft-campaign", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.YourProgram as Program<AffiliateDapp>;

  const campaignName = "test-campaign";
  const mintPrice = new anchor.BN(1_000_000); // 1 SOL in lamports
  const commissionPercentage = 10;
  const campaignDetails = "Test campaign details";

  let company = Keypair.generate();
  let nftMint = Keypair.generate();
  let companyTokenAccount: PublicKey;
  let campaignPda: PublicKey;
  let nftEscrowPda: PublicKey;
  let escrowTokenAccount: PublicKey;

  before(async () => {
    // Airdrop SOL to company
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(company.publicKey, 1e9),
      "confirmed"
    );

    // Create NFT Mint using Token-2022
    const mintSpace = 82; // Size for SPL Token Mint
    const mintLamports = await provider.connection.getMinimumBalanceForRentExemption(mintSpace);
    const createMintTx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: provider.publicKey,
        newAccountPubkey: nftMint.publicKey,
        space: mintSpace,
        lamports: mintLamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        nftMint.publicKey,
        0, // Decimals for NFT
        company.publicKey, // Mint Authority
        null, // Freeze Authority
        TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(createMintTx, [nftMint]);

    // Create company's token account
    companyTokenAccount = getAssociatedTokenAddressSync(
      nftMint.publicKey,
      company.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );
    const createATAInstruction = createAssociatedTokenAccountInstruction(
      company.publicKey, 
      companyTokenAccount,
      company.publicKey,
      nftMint.publicKey,
      TOKEN_2022_PROGRAM_ID
    );
    await provider.sendAndConfirm(new anchor.web3.Transaction().add(createATAInstruction),[company]);

    // Mint 1 NFT to company
    await provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        createMintToInstruction(
          nftMint.publicKey,
          companyTokenAccount,
          company.publicKey,
          1,
          [],
          TOKEN_2022_PROGRAM_ID
        )
      ),[company]
    );

    // Derive campaign PDA
    [campaignPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("nft_campaign"), Buffer.from(campaignName)],
      program.programId
    );

    // Derive NFT escrow PDA (token account)
    [nftEscrowPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("nft_escrow"), campaignPda.toBuffer()],
      program.programId
    );

    // Create escrow's token account (if needed, depends on instruction)
    // Note: This step might not be necessary if instruction initializes it
  });

  it("Successfully creates NFT campaign", async () => {
    // Derive escrow's associated token account
    escrowTokenAccount = getAssociatedTokenAddressSync(
      nftMint.publicKey,
      nftEscrowPda,
      true,
      TOKEN_2022_PROGRAM_ID
    );

    await program.methods
      .createNftCampaign(
        campaignName,
        mintPrice,
        commissionPercentage,
        campaignDetails
      )
      .accounts({
        company: company.publicKey,
        nftMint: nftMint.publicKey,
        campaign: campaignPda,
        nftEscrow: nftEscrowPda,
        escrowTokenAccount,
        companyTokenAccount,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([company])
      .rpc();

    // Verify campaign account
    const campaignAccount = await program.account.nftCampaign.fetch(campaignPda);
    assert.equal(campaignAccount.name, campaignName);
    assert.equal(campaignAccount.mintPrice.toString(), mintPrice.toString());
    assert.equal(campaignAccount.commissionPercentage, commissionPercentage);
    assert.equal(campaignAccount.campaignDetails, campaignDetails);
    assert.isTrue(campaignAccount.active);

    // Verify NFT transfer to escrow
    const escrowAccount = await getAccount(
      provider.connection,
      escrowTokenAccount,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );
    assert.equal(escrowAccount.amount, BigInt(1));

    // Verify company's token account is empty
    const companyAccount = await getAccount(
      provider.connection,
      companyTokenAccount,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );
    assert.equal(companyAccount.amount, BigInt(0));
  });

  // Additional tests for constraints
  it("Fails with name longer than 32 chars", async () => {
    const longName = "a".repeat(33);
    try {
      await program.methods
        .createNftCampaign(longName, mintPrice, commissionPercentage, campaignDetails)
        .accounts({ 
          company: company.publicKey,
         })
        .signers([company])
        .rpc();
      assert.fail("Expected error for long name");
    } catch (err) {
      console.error(err);
      assert.include(err.message, "name.len <= 32");
    }
  });

  it("Fails if company is not mint authority", async () => {
    const fakeCompany = Keypair.generate();
    try {
      await program.methods
        .createNftCampaign(campaignName, mintPrice, commissionPercentage, campaignDetails)
        .accounts({ 
          company: company.publicKey,
          nftMint: nftMint.publicKey,
          campaign: campaignPda,
          nftEscrow: nftEscrowPda,
          escrowTokenAccount,
          companyTokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
        })
        .signers([fakeCompany])
        .rpc();
      assert.fail("Expected error for wrong mint authority");
    } catch (err) {
      console.error(err);
      assert.include(err.message, "constraint");
    }
  });
});