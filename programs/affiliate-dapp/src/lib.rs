use anchor_lang::prelude::*;
mod state;
mod instructions;
use instructions::*;
//use instructions::{create_nft_campaign_instruction};
declare_id!("57HuJeX7Qm7Vv1zYLP3WtYCAbEqMBEo5AD1d2jaEKNxK");

#[program]

pub mod affiliate_dapp {
    use super::*;
    pub fn create_nft_campaign(
        ctx: Context<CreateNFTCampaign>,
        name: String,
        mint_price: u64,
        commission_percentage: u8,
        campaign_details: String,
    ) -> Result<()> {
        create_nft_campaign_instruction(ctx, name, mint_price, commission_percentage, campaign_details)
    }
    pub fn create_affiliate_link(
        ctx: Context<CreateAffiliateLink>,
        campaign_id: Pubkey,
        campaign_name: String,
    ) -> Result<()> {
        create_affiliate_link_instruction(ctx, campaign_id,campaign_name)
    }
    pub fn process_affiliate_mint(
        ctx: Context<ProcessAffiliateMint>,
    ) -> Result<()> {
        process_affiliate_mint_instruction(ctx)
    }

   
}