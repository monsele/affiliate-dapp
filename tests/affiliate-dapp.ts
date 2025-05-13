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
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";
import { AffiliateDapp } from "../target/types/affiliate_dapp"; // Replace with your program type

describe("nft-campaign", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.AffiliateDapp as Program<AffiliateDapp>;

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
  let influencer = Keypair.generate();
  let affiliateLinkPda: PublicKey;
  before(async () => {
    // Airdrop SOL to company
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(company.publicKey, 1e9),
      "confirmed"
    );
     await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(influencer.publicKey, 1e9),
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

     [affiliateLinkPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("affiliate_link"),
        influencer.publicKey.toBuffer(),
        Buffer.from(campaignName),
      ],
      program.programId
    );
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
        campaign: campaignPda,
        nftMint: nftMint.publicKey,
        projectTokenAccount: companyTokenAccount,
        nftEscrow: nftEscrowPda,
        escrowPdaNftTokenAccount: escrowTokenAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram:  ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
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
  it("Creates an affiliate link successfully", async () => {
  await program.methods
    .createAffiliateLink(campaignName)  // Only pass the campaign name parameter
    .accounts({
      affiliateLink: affiliateLinkPda,
      campaign: campaignPda,
      influencer: influencer.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([influencer])
    .rpc();

  // Fetch and assert affiliate link state
  const affiliateLinkAccount = await program.account.affiliateLink.fetch(affiliateLinkPda);
  assert.equal(affiliateLinkAccount.influencer.toBase58(), influencer.publicKey.toBase58());
  assert.equal(affiliateLinkAccount.campaign.toBase58(), campaignPda.toBase58());
  assert.equal(affiliateLinkAccount.mintsCount.toString(), "0");
  assert.equal(affiliateLinkAccount.earnings.toString(), "0");
  assert.isTrue(affiliateLinkAccount.createdAt.toNumber() > 0); // Verify timestamp was set

  // Fetch campaign to verify affiliates_count incremented
  const campaignAccount = await program.account.nftCampaign.fetch(campaignPda);
  assert.equal(campaignAccount.affiliatesCount.toString(), "1");
  });



  it("Processes affiliate mint correctly", async () => {
  const buyer = Keypair.generate();
  const buyerAta = getAssociatedTokenAddressSync(
    nftMint.publicKey,
    buyer.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID
  );

  // Airdrop SOL to buyer for minting and fees
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(buyer.publicKey, 2e9),
    "confirmed"
  );

  // Derive marketplace authority PDA
  const [marketplaceAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("marketplace_authority")],
    program.programId
  );
  const influencerBalanceBefore = await provider.connection.getBalance(influencer.publicKey);
  const companyBalanceBefore = await provider.connection.getBalance(company.publicKey);
  console.log("Influencer Balance Before: ", influencerBalanceBefore);
  console.log("Company Balance Before: ", companyBalanceBefore);

  // Execute process_affiliate_mint
  await program.methods
    .processAffiliateMint(campaignName,influencer.publicKey)
    .accounts({
      campaign: campaignPda,
      affiliateLink: affiliateLinkPda,
      buyer: buyer.publicKey,
      owner: company.publicKey,
      influencer: influencer.publicKey,
      nftMint: nftMint.publicKey,
      nftEscrow: nftEscrowPda,
      buyerTokenAccount: buyerAta,
      ownerTokenAccount: companyTokenAccount,
      escrowPdaNftTokenAccount: escrowTokenAccount,
      marketplaceAuthority: marketplaceAuthorityPda,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([buyer])
    .rpc();

  // Verify influencer received commission
  const influencerBalanceAfter = await provider.connection.getBalance(influencer.publicKey);
  console.log("Influencer Balance After: ", influencerBalanceAfter);
  assert.isAbove(influencerBalanceAfter,influencerBalanceBefore , "Influencer should receive commission");
  //assert.isAbove(influencerBalanceAfter, 1e9, "Influencer should receive commission");

  // Verify company received the project amount
  const companyBalanceAfter = await provider.connection.getBalance(company.publicKey);
  assert.isAbove(companyBalanceAfter, companyBalanceBefore, "Company should receive project amount");

  // Verify buyer received NFT
  const buyerAccount = await getAccount(
    provider.connection,
    buyerAta,
    "confirmed",
    TOKEN_2022_PROGRAM_ID
  );
  assert.equal(buyerAccount.amount, BigInt(1), "Buyer should receive 1 NFT");

  // Verify escrow token account is empty
  const escrowAccountAfter = await getAccount(
    provider.connection,
    escrowTokenAccount,
    "confirmed",
    TOKEN_2022_PROGRAM_ID
  );
  assert.equal(escrowAccountAfter.amount, BigInt(0), "Escrow account should be empty");

  // Verify affiliate link state updated
  const affiliateLinkAccount = await program.account.affiliateLink.fetch(affiliateLinkPda);
  assert.equal(affiliateLinkAccount.mintsCount.toString(), "1");
  assert.equal(affiliateLinkAccount.earnings.toString(), (mintPrice.toNumber() * commissionPercentage / 100).toString());

  // Verify campaign state updated
  const campaignAccount = await program.account.nftCampaign.fetch(campaignPda);
  assert.equal(campaignAccount.totalMints.toString(), "1");
});

  
});
  
