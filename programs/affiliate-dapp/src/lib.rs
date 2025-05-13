use anchor_lang::prelude::*;
mod state;
mod instructions;
use instructions::*;
mod error;
//use instructions::{create_nft_campaign_instruction};
declare_id!("Fqem6roKkvhpFtHuHDTXDzcGD9zpEgH7zVCi3Sf4hUFb");

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
        campaign_name: String,
    ) -> Result<()> {
        create_affiliate_link_instruction(ctx,campaign_name)
    }
    pub fn process_affiliate_mint(
        ctx: Context<ProcessAffiliateMint>,campaign_name: String, influencer: Pubkey
    ) -> Result<()> {
        process_affiliate_mint_instruction(ctx,campaign_name, influencer)
    }

   
}