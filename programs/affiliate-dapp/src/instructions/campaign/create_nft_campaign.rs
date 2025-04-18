
use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(name: String, mint_price: u64, commission_percentage: u8, campaign_details: String)]
pub struct CreateNFTCampaign<'info> {
    #[account(
        init,
        payer = company,
        space = 8 + 32 + 32 + 32 + (4 + name.len()) + 8 + 1 + (4 + campaign_details.len()) + 1 + 8 + 8
    )]
    pub campaign: Account<'info, NFTCampaign>,
    
    // NFT project's main wallet
    /// CHECK: This account is not being read or written
    pub nft_project: UncheckedAccount<'info>,
    
    
    // The program ID that handles minting for this NFT collection
    /// CHECK: This account is not being read or written
    pub nft_mint_program: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub company: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_nft_campaign_instruction(
    ctx: Context<CreateNFTCampaign>,
    name: String,
    mint_price: u64,
    commission_percentage: u8,
    campaign_details: String,
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    campaign.nft_project =ctx.accounts.nft_project.key(); 
    campaign.nft_mint_program = ctx.accounts.nft_mint_program.key();
    campaign.company = ctx.accounts.company.key();
    campaign.name = name;
    campaign.mint_price = mint_price;
    campaign.commission_percentage = commission_percentage;
    campaign.campaign_details = campaign_details;
    campaign.active = true;
    campaign.affiliates_count = 0;
    campaign.total_mints = 0;
    
    Ok(())
}
