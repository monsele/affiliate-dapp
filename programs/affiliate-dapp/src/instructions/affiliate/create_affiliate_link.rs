use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
#[instruction(campaign_name: String)]  // Remove campaign_id parameter if not used
pub struct CreateAffiliateLink<'info> {
    #[account(
        init,
        payer = influencer,
        space = 8 + 32 + 32 + 8 + 8 + 8,  // 8 (discriminator) + struct size (including created_at)
        seeds = [b"affiliate_link", influencer.key().as_ref(), campaign_name.as_bytes()],
        bump,
    )]
    pub affiliate_link: Account<'info, AffiliateLink>,
    
    #[account(
        mut,  // Mark as mutable since we're updating affiliates_count
        seeds = [b"nft_campaign", campaign_name.as_bytes()],
        bump,
    )]
    pub campaign: Account<'info, NFTCampaign>,
    
    #[account(mut)]
    pub influencer: Signer<'info>,
    
    pub system_program: Program<'info, System>
}

// And update your instruction handler
pub fn create_affiliate_link_instruction(
    ctx: Context<CreateAffiliateLink>,
    _campaign_name: String,  // Keep the campaign name parameter
) -> Result<()> {
    let affiliate_link = &mut ctx.accounts.affiliate_link;
    let campaign = &mut ctx.accounts.campaign;
    
    affiliate_link.campaign = campaign.key();
    affiliate_link.influencer = ctx.accounts.influencer.key();
    affiliate_link.mints_count = 0;
    affiliate_link.earnings = 0;
    affiliate_link.created_at = Clock::get()?.unix_timestamp;  // Set creation timestamp
    
    campaign.affiliates_count += 1;
    
    Ok(())
}