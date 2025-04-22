import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { AffiliateDapp } from "../target/types/affiliate_dapp";
import { Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import { assert } from "chai";

describe("affiliate-dapp", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AffiliateDapp as Program<AffiliateDapp>;

  let company = provider.wallet;
  let influencer = Keypair.generate();
  
  let buyer = Keypair.generate();

  let nftProject = Keypair.generate();
  let nftMintProgram = Keypair.generate();

  //let campaign = Keypair.generate();
  //let affiliateLink = Keypair.generate();

  const campaignName = "Test Campaign";
  const mintPrice = new anchor.BN(1_000_000_000); // 1 SOL
  const commissionPercentage = 10; // 10%
  const campaignDetails = "Promo campaign for NFTs";
  const getCampaignPDA = (name: string) => {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from("nft_campaign"),
        Buffer.from(name),
      ],
      program.programId
    )[0];
  };
  const getAffiliateLinkPDA = (name: string) => {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from("affiliate_link"),
        influencer.publicKey.toBuffer(),
        Buffer.from(name),
      ],
      program.programId
    )[0];
  };
  before(async () => {
    // Airdrop SOL to influencer and buyer
    for (const user of [influencer, buyer]) {
      const tx = await provider.connection.requestAirdrop(user.publicKey, 2_000_000_000);
      await provider.connection.confirmTransaction(tx);
    }
  });

  it("Creates a new NFT campaign", async () => {
    const campaignPDA = getCampaignPDA(campaignName);

    await program.methods
      .createNftCampaign(
        campaignName,
        mintPrice,
        commissionPercentage,
        campaignDetails
      )
      .accounts({
        campaign: campaignPDA, // Use PDA instead of random keypair
        nftProject: nftProject.publicKey,
        nftMintProgram: nftMintProgram.publicKey,
        company: company.publicKey,
        systemProgram: SystemProgram.programId,
      })
      // Remove .signers([buyer]) 👇
      .rpc();

    const campaignAccount = await program.account.nftCampaign.fetch(campaignPDA);

    assert.equal(campaignAccount.name, campaignName);
    assert.equal(campaignAccount.mintPrice.toNumber(), mintPrice.toNumber());
    assert.equal(campaignAccount.commissionPercentage, commissionPercentage);
    assert.isTrue(campaignAccount.active);
  });

  it("Creates an affiliate link", async () => {
    let linkPDA = getAffiliateLinkPDA(campaignName);
    await program.methods
      .createAffiliateLink(getCampaignPDA(campaignName),campaignName)
      .accounts({
        affiliateLink: linkPDA,
        campaign: getCampaignPDA(campaignName),
        influencer: influencer.publicKey,
        system_Program: SystemProgram.programId,
      })
      .signers([influencer])
      .rpc();

    const link = await program.account.affiliateLink.fetch(linkPDA);
    assert.equal(link.influencer.toBase58(), influencer.publicKey.toBase58());
    //assert.equal(link.campaign.toBase58(), campaign.publicKey.toBase58());
  });

  // it("Processes an affiliate mint", async () => {
  //   const commissionAmount = mintPrice.toNumber() * commissionPercentage / 100;
  //   const projectAmount = mintPrice.toNumber() - commissionAmount;

  //   const initialInfluencerBalance = await provider.connection.getBalance(influencer.publicKey);
  //   const initialProjectBalance = await provider.connection.getBalance(nftProject.publicKey);

  //   const dummyIxData = Buffer.from([]); // You'd usually provide real instruction data

  //   await program.methods
  //     .processAffiliateMint(dummyIxData)
  //     .accounts({
  //       campaign: campaign.publicKey,
  //       affiliateLink: affiliateLink.publicKey,
  //       buyer: buyer.publicKey,
  //       influencer: influencer.publicKey,
  //       owner: company.publicKey,
  //       systemProgram: SystemProgram.programId,
  //     })
  //     .remainingAccounts([]) // No real mint instruction here
  //     .signers([buyer])
  //     .rpc();

  //   const finalInfluencerBalance = await provider.connection.getBalance(influencer.publicKey);
  //   const finalProjectBalance = await provider.connection.getBalance(nftProject.publicKey);

  //   assert.isAbove(finalInfluencerBalance, initialInfluencerBalance);
  //   assert.isAbove(finalProjectBalance, initialProjectBalance);

  //   const updatedCampaign = await program.account.nftCampaign.fetch(campaign.publicKey);
  //   const updatedAffiliate = await program.account.affiliateLink.fetch(affiliateLink.publicKey);

  //   assert.equal(updatedCampaign.totalMints, 1);
  //   assert.equal(updatedAffiliate.mintsCount, 1);
  //   assert.equal(updatedAffiliate.earnings.toNumber(), commissionAmount);
  // });
});
