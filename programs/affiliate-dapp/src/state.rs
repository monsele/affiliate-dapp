use anchor_lang::prelude::*;
//use anchor_lang::solana_program::pubkey::Pubkey;

#[account]
#[derive(InitSpace,Debug)]
pub struct NFTCampaign {
    pub nft_mint: Pubkey,             // Mint account of the NFT project
    pub nft_mint_program: Pubkey,     // Program ID that handles minting
    pub company: Pubkey,              // Campaign creator
    #[max_len(32)]
    pub name: String,                 // Campaign name
    pub mint_price: u64,              // Price to mint one NFT
    pub commission_percentage: u8,    // Commission percentage (0-100)
    #[max_len(200)]
    pub campaign_details: String,     // Additional details
    pub active: bool,                 // Campaign status
    pub affiliates_count: u64,        // Number of affiliates
    pub total_mints: u64,             // Total NFTs minted via affiliates
    pub created_at: i64,             // Timestamp of campaign creation
}

#[account]
#[derive(InitSpace,Debug)]
pub struct AffiliateLink {
    pub campaign: Pubkey,        // Campaign this affiliate link belongs to
    pub influencer: Pubkey,      // Wallet of the influencer
    pub mints_count: u64,        // Number of mints through this link
    pub earnings: u64,           // Total earnings in lamports
   // pub influencer_token_account: Pubkey, // Token account of the influencer
    pub created_at: i64,         // Timestamp of link creation
}