use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(campaign_id: Pubkey, campaign_name: String)]
pub struct CreateAffiliateLink<'info> {
    #[account(
        init,
        payer = influencer,
        space = 8 + 32 + 32 + 8 + 8,
        seeds = [b"affiliate_link", influencer.key().as_ref(), campaign_name.as_bytes()],
        bump,
    )]
    pub affiliate_link: Account<'info, AffiliateLink>,
    
    #[account(
        seeds = [b"nft_campaign", campaign_name.as_bytes()],
        bump,
    )]
    pub campaign: Account<'info, NFTCampaign>,
    
    // NFT Owner 
    //pub nft_owner: UncheckedAccount<'info>,
    #[account(mut)]
    pub influencer: Signer<'info>,
    
    pub system_program: Program<'info, System>
}

pub fn create_affiliate_link_instruction(
    ctx: Context<CreateAffiliateLink>,
    _campaign_id: Pubkey, _campaign_name: String,
) -> Result<()> {
    let affiliate_link = &mut ctx.accounts.affiliate_link;
    let campaign = &mut ctx.accounts.campaign;
    
    affiliate_link.campaign = campaign.key();
    affiliate_link.influencer = ctx.accounts.influencer.key();
    affiliate_link.mints_count = 0;
    affiliate_link.earnings = 0;
    
    campaign.affiliates_count += 1;
    
    Ok(())
}
