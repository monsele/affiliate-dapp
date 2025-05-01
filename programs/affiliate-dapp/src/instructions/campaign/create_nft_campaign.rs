
//use anchor_lang::prelude::*;
use {
    anchor_lang::prelude::*,
    anchor_spl::{ token_interface::{Mint, TokenAccount, TokenInterface,  spl_token_2022::ID as TOKEN_2022_PROGRAM_ID}}
};
use anchor_spl::associated_token::AssociatedToken;
use anchor_lang::solana_program::program_option::COption;
use crate::state::*;
// Get NFT Mint details and approve 
#[derive(Accounts)]
#[instruction(name: String, mint_price: u64, commission_percentage: u8, campaign_details: String)]
pub struct CreateNFTCampaign<'info> {
    #[account(
        init,
        payer = company,
        space = 8 + 32 + 32 + 32 + (4 + name.len()) + 8 + 1 + (4 + campaign_details.len()) + 1 + 8 + 8,
        seeds = [b"nft_campaign", name.as_bytes()],
        bump,
        constraint = name.len() <= 32,
        constraint = campaign_details.len() <= 200,
    )]
    pub campaign: Account<'info, NFTCampaign>,
    
    #[account(
        mut,
        constraint = nft_mint.mint_authority == COption::Some(company.key()),
        owner = TOKEN_2022_PROGRAM_ID  // ← Critical for Token-2022
    )]
    pub nft_mint: InterfaceAccount<'info, Mint>,
    
    // The program ID that handles minting for this NFT collection
    /// CHECK: This account is not being read or written
    pub nft_mint_program: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = company,
        associated_token::token_program = token_program,
    )]
    pub company_token_account: InterfaceAccount<'info, TokenAccount>,
    
    #[account(mut)]
    pub company: Signer<'info>,
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
// // Event emitted when an NFT is listed
// #[event]
// pub struct ListingCreatedEvent {
//     pub listing: Pubkey,
//     pub seller: Pubkey,
//     pub nft_mint: Pubkey,
//     pub price: u64,
//     pub created_at: i64,
// }

// // Event emitted when an NFT is sold
// #[event]
// pub struct SaleExecutedEvent {
//     pub listing: Pubkey,
//     pub seller: Pubkey,
//     pub buyer: Pubkey,
//     pub nft_mint: Pubkey,
//     pub price: u64,
//     pub sold_at: i64,
// }

// // Event emitted when a listing is canceled
// #[event]
// pub struct ListingCanceledEvent {
//     pub listing: Pubkey,
//     pub seller: Pubkey,
//     pub nft_mint: Pubkey,
//     pub canceled_at: i64,
// }

pub fn create_nft_campaign_instruction(
    ctx: Context<CreateNFTCampaign>,
    name: String,
    mint_price: u64,
    commission_percentage: u8,
    campaign_details: String,
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    campaign.nft_mint =ctx.accounts.nft_mint.key(); 
    campaign.nft_mint_program = ctx.accounts.nft_mint_program.key();
    campaign.company = ctx.accounts.company.key();
    campaign.name = name;
    campaign.mint_price = mint_price;
    campaign.commission_percentage = commission_percentage;
    campaign.campaign_details = campaign_details;
    campaign.active = true;
    campaign.affiliates_count = 0;
    campaign.total_mints = 0;
    campaign.created_at = Clock::get()?.unix_timestamp;
    // emit!(ListingCreatedEvent {
    //     listing: campaign.key(),
    //     seller: campaign.company.key(),
    //     nft_mint:campaign.nft_mint.key(),
    //     price: mint_price,
    //     created_at: campaign.created_at,
    // });
    Ok(())
}
