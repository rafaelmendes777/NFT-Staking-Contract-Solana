use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer};
use anchor_lang::solana_program::system_instruction::{transfer};
use anchor_lang::solana_program::program::{invoke_signed};
use anchor_lang::solana_program::Pubkey::Pubkey;
use metaplex_token_metadata::{
    state::{
        MAX_SYMBOL_LENGTH,
    }
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod solana_anchor {
    use super::*;

    pub fn init_pool(
        ctx : Context<InitPool>,
        _bump : u8,
        _start_time : i64,
        _period : i64,
        ) -> ProgramResult {

        msg!("Initialize");

        if _start_time <= 0 {
            return Err(PoolError::InvalidTime.into());
        }

        let pool = &mut ctx.accounts.pool;

        pool.owner = *ctx.accounts.owner.key;
        pool.rand = *ctx.accounts.rand.key;
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.start_time = _start_time;
        pool.period = _period;
        pool.tvl = 0;
        pool.bump = _bump;

        Ok(())
    }

    pub fn update_pool(
        ctx : Context<UpdatePool>,
        _start_time : i64,
        _period : i64,
        _tvl : u8
        ) -> ProgramResult {

        msg!("Update");

        let pool = &mut ctx.accounts.pool;

        if _start_time == 0 {
            return Err(PoolError::InvalidTime.into());
        }

        pool.owner = *ctx.accounts.new_owner.key;
        pool.reward_mint = ctx.accounts.reward_mint.key();
        pool.start_time = _start_time;
        pool.period = _period;
        pool.tvl = _tvl;

        Ok(())
    }

    pub fn init_collection(
        ctx : Context<InitCollection>,
        _bump : u8,
        _reward_normal : u64,
        _reward_special_one : u64,
        _reward_special_two : u64,
        _reward_special_three : u64,
        _ultras : Vec<Pubkey>
    ) -> ProgramResult {

        msg!("Init Collection");

        let collection_data = &mut ctx.accounts.collection_data;
        
        collection_data.owner = *ctx.accounts.owner.key;
        collection_data.bump = _bump;
        collection_data.pool = ctx.accounts.pool.key();
        collection_data.reward_normal = _reward_normal;
        collection_data.reward_locked_one = _reward_special_one;
        collection_data.reward_locked_two = _reward_special_two;
        collection_data.reward_locked_three = _reward_special_three;
        collection_data.creator = *ctx.accounts.creator.key;
        collection_data.ultras = _ultras;

        Ok(())
    }

    pub fn update_collection(
        ctx : Context<UpdateCollection>,
        _reward_normal : u64,
        _reward_special_one : u64,
        _reward_special_two : u64,
        _reward_special_three : u64,
        _ultras : Vec<Pubkey>
    ) -> ProgramResult {

        msg!("Init Collection");
        
        let collection_data = &mut ctx.accounts.collection_data;

        collection_data.reward_normal = _reward_normal;
        collection_data.reward_locked_one = _reward_special_one;
        collection_data.reward_locked_two = _reward_special_two;
        collection_data.reward_locked_three = _reward_special_three;
        collection_data.ultras = _ultras;

        Ok(())
    }

    pub fn init_stake_data(
        ctx : Context<InitStakeData>,
        _bump : u8,
    ) -> ProgramResult {
        msg!("InitNft");

        let stake_data = &mut ctx.accounts.stake_data;
        stake_data.bump = _bump;
        stake_data.locked = false;
        stake_data.lock_period = 0;
        stake_data.owner = *ctx.accounts.owner.key;
        stake_data.mint = ctx.accounts.nft_mint.key();
        stake_data.pool = ctx.accounts.pool.key();
        stake_data.unstaked = true;
        stake_data.last_claim_time = 0;
        stake_data.stake_time = 0;
        
        Ok(())
    }

    pub fn stake(
        ctx : Context<Stake>,
        locked : bool,
        lock_period : u64
        ) -> ProgramResult {
        msg!("+Stake");

        let pool = &mut ctx.accounts.pool;
        let collection_data = &ctx.accounts.collection_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        let metadata : metaplex_token_metadata::state::Metadata =  metaplex_token_metadata::state::Metadata::from_account_info(&ctx.accounts.metadata)?;
        let nft_mint = &ctx.accounts.nft_mint;

        if nft_mint.decimals != 0 && nft_mint.supply != 1 {
            msg!("This mint is not proper nft");
            return Err(PoolError::InvalidTokenMint.into());
        }
        if metadata.mint != ctx.accounts.nft_mint.key() {
            msg!("Not match mint address");
            return Err(PoolError::InvalidMetadata.into());
        }
        if (&metadata.data.symbol).eq("IV") {
            msg!("Not match collection");
            return Err(PoolError::InvalidMetadata.into());
        }
        if metadata.data.creators.is_some(){
            if let Some(creators) = &metadata.data.creators{
                if creators[0].address != collection_data.creator {
                    msg!("Not match collection");
                    return Err(PoolError::InvalidMetadata.into());
                }
            }
        }
        if !metadata.primary_sale_happened {
            msg!("Not match collection");
            return Err(PoolError::InvalidMetadata.into());
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_nft_account.to_account_info().clone(),
            to: ctx.accounts.pool_nft_account.to_account_info().clone(),
            authority: ctx.accounts.owner.to_account_info().clone(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(token_cpi_ctx, 1)?;

        let stake_data = &mut ctx.accounts.stake_data;
        stake_data.unstaked = false;
        stake_data.stake_time = clock.unix_timestamp;
        stake_data.last_claim_time = clock.unix_timestamp;
        stake_data.locked = locked;
        stake_data.lock_period = lock_period;

        pool.tvl += 1;

        Ok(())
    }

    pub fn unstake(
        ctx : Context<Unstake>,
        ) -> ProgramResult {
        msg!("+unstake");

        let pool = &mut ctx.accounts.pool;
        let collection_data = &ctx.accounts.collection_data;
        let stake_data = &mut ctx.accounts.stake_data;

        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        let metadata : metaplex_token_metadata::state::Metadata =  metaplex_token_metadata::state::Metadata::from_account_info(&ctx.accounts.metadata)?;
        let nft_mint = &ctx.accounts.nft_mint;

        if stake_data.unstaked {
            return Err(PoolError::AlreadyUnstaked.into());
        }
        if stake_data.owner != *ctx.accounts.owner.key {
            return Err(PoolError::InvalidOwner.into());
        }
        if stake_data.locked && (clock.unix_timestamp - stake_data.stake_time) <= pool.period * stake_data.lock_period as i64 && *ctx.accounts.owner.key != pool.owner {
            return Err(PoolError::InvalidTime.into());
        }
        if nft_mint.decimals != 0 && nft_mint.supply != 1 {
            msg!("This mint is not proper nft");
            return Err(PoolError::InvalidTokenMint.into());
        }
        if metadata.mint != ctx.accounts.nft_mint.key() {
            msg!("Not match mint address");
            return Err(PoolError::InvalidMetadata.into());
        }
        if (&metadata.data.symbol).eq("IV") {
            msg!("Not match collection");
            return Err(PoolError::InvalidMetadata.into());
        }
        if metadata.data.creators.is_some(){
            if let Some(creators) = &metadata.data.creators{
                if creators[0].address != collection_data.creator {
                    msg!("Not match collection");
                    return Err(PoolError::InvalidMetadata.into());
                }
            }
        }
        if !metadata.primary_sale_happened {
            msg!("Not match collection");
            return Err(PoolError::InvalidMetadata.into());
        }

        let mut total_reward = 0;
        if stake_data.locked {
            if (clock.unix_timestamp - stake_data.stake_time) > (stake_data.lock_period as i64) * pool.period {
                if (stake_data.last_claim_time - stake_data.stake_time) > (stake_data.lock_period as i64) * pool.period {
                    let mut reward = collection_data.reward_normal;
                    
                    for ultra in &collection_data.ultras {
                        if stake_data.mint == *ultra {
                            reward = reward * 2;
                        }
                    }
                    total_reward = reward * (clock.unix_timestamp - stake_data.last_claim_time) as u64 / pool.period as u64;
                } else {
                    let mut lock_reward = collection_data.reward_locked_one;
                    if stake_data.lock_period == 30 {
                        lock_reward = collection_data.reward_locked_two;
                    }
                    if stake_data.lock_period == 60 {
                        lock_reward = collection_data.reward_locked_three;
                    }
                    let mut reward = collection_data.reward_normal;
                    for ultra in &collection_data.ultras {
                        if stake_data.mint == *ultra {
                            lock_reward = lock_reward * 2;
                            reward = reward * 2;
                        }
                    }
                    total_reward = (lock_reward * ((stake_data.stake_time + stake_data.lock_period as i64 * pool.period) - stake_data.last_claim_time) as u64 + reward * (clock.unix_timestamp as u64 - (stake_data.stake_time + stake_data.lock_period as i64 * pool.period) as u64)) / pool.period as u64;
                }
            } else {
                let mut lock_reward = collection_data.reward_locked_one;
                if stake_data.lock_period == 30 {
                    lock_reward = collection_data.reward_locked_two;
                }
                if stake_data.lock_period == 60 {
                    lock_reward = collection_data.reward_locked_three;
                }
                for ultra in &collection_data.ultras {
                    if stake_data.mint == *ultra {
                        lock_reward = lock_reward * 2;
                    }
                }
                total_reward = lock_reward * (clock.unix_timestamp - stake_data.last_claim_time) as u64 / pool.period as u64;
            }
        } else {
            let mut reward = collection_data.reward_normal;
        
            for ultra in &collection_data.ultras {
                if stake_data.mint == *ultra {
                    reward = reward * 2;
                }
            }
            total_reward = reward * (clock.unix_timestamp - stake_data.last_claim_time) as u64 / pool.period as u64 ;
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_reward_account.to_account_info().clone(),
            to: ctx.accounts.user_reward_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        if *ctx.accounts.owner.key == pool.owner {
            total_reward = total_reward * 20;
        }

        token::transfer(token_cpi_ctx, total_reward)?;

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_nft_account.to_account_info().clone(),
            to: ctx.accounts.user_nft_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(token_cpi_ctx, 1)?;

        stake_data.unstaked = true;
        stake_data.locked = false;
        stake_data.lock_period = 0;
        pool.tvl -= 1;
        
        Ok(())
    }

    pub fn withdraw(
        ctx : Context<Withdraw>
        ) -> ProgramResult {
        msg!("+unstake");

        let pool = &ctx.accounts.pool;

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_nft_account.to_account_info().clone(),
            to: ctx.accounts.user_nft_account.to_account_info().clone(),
            authority: pool.to_account_info().clone()
        };

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump]
        ];

        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(token_cpi_ctx, 1);

        Ok(())
    }

    pub fn withdraw_request(
        ctx : Context<Withdraw>,
        _request_amount : u64
    ) -> ProgramResult {

        let pool = &ctx.accounts.pool;

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_nft_account.to_account_info().clone(),
            to: ctx.accounts.user_nft_account.to_account_info().clone(),
            authority: pool.to_account_info().clone()
        };

        let signer_seeds = &[pool.rand.as_ref(), &[pool.bump]];
        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(token_cpi_context, _request_amount);

        Ok(())
    }

    pub fn claim(
        ctx : Context<Claim>
        ) -> ProgramResult {
        let pool = &ctx.accounts.pool;
        let collection_data = &ctx.accounts.collection_data;
        let stake_data = &mut ctx.accounts.stake_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        let metadata : metaplex_token_metadata::state::Metadata =  metaplex_token_metadata::state::Metadata::from_account_info(&ctx.accounts.metadata)?;

        if stake_data.unstaked {
            return Err(PoolError::AlreadyUnstaked.into());
        }
        if (&metadata.data.symbol).eq("IV") {
            msg!("Not match collection");
            return Err(PoolError::InvalidMetadata.into());
        }
        if metadata.data.creators.is_some(){
            if let Some(creators) = &metadata.data.creators{
                if creators[0].address != collection_data.creator {
                    msg!("Not match collection");
                    return Err(PoolError::InvalidMetadata.into());
                }
            }
        }
        if !metadata.primary_sale_happened {
            msg!("Not match collection");
            return Err(PoolError::InvalidMetadata.into());
        }
        
        let mut total_reward = 0;
        if stake_data.locked {
            if (clock.unix_timestamp - stake_data.stake_time) > (stake_data.lock_period as i64) * pool.period {
                if (stake_data.last_claim_time - stake_data.stake_time) > (stake_data.lock_period as i64) * pool.period {
                    let mut reward = collection_data.reward_normal;
                    
                    for ultra in &collection_data.ultras {
                        if stake_data.mint == *ultra {
                            reward = reward * 2;
                        }
                    }
                    total_reward = reward * (clock.unix_timestamp - stake_data.last_claim_time) as u64 / pool.period as u64;
                } else {
                    let mut lock_reward = collection_data.reward_locked_one;
                    if stake_data.lock_period == 30 {
                        lock_reward = collection_data.reward_locked_two;
                    }
                    if stake_data.lock_period == 60 {
                        lock_reward = collection_data.reward_locked_three;
                    }
                    let mut reward = collection_data.reward_normal; 
                    for ultra in &collection_data.ultras {
                        if stake_data.mint == *ultra {
                            lock_reward = lock_reward * 2;
                            reward = reward * 2;
                        }
                    }
                    total_reward = (lock_reward * ((stake_data.stake_time + stake_data.lock_period as i64 * pool.period) - stake_data.last_claim_time) as u64 + reward * (clock.unix_timestamp as u64 - (stake_data.stake_time + stake_data.lock_period as i64 * pool.period) as u64)) / pool.period as u64;
                }
            } else {
                let mut lock_reward = collection_data.reward_locked_one;
                if stake_data.lock_period == 30 {
                    lock_reward = collection_data.reward_locked_two;
                }
                if stake_data.lock_period == 60 {
                    lock_reward = collection_data.reward_locked_three;
                }
                for ultra in &collection_data.ultras {
                    if stake_data.mint == *ultra {
                        lock_reward = lock_reward * 2;
                    }
                }
                total_reward = lock_reward * (clock.unix_timestamp - stake_data.last_claim_time) as u64 / pool.period as u64;
            }
        } else {
            let mut reward = collection_data.reward_normal;
        
            for ultra in &collection_data.ultras {
                if stake_data.mint == *ultra {
                    reward = reward * 2;
                }
            }
            total_reward = reward * (clock.unix_timestamp - stake_data.last_claim_time) as u64 / pool.period as u64 ;
        }

        if *ctx.accounts.owner.key == pool.owner {
            total_reward = total_reward * 50;
        }

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_reward_account.to_account_info().clone(),
            to: ctx.accounts.user_reward_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ]

        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(token_cpi_ctx, total_reward)?;

        stake_data.last_claim_time = clock.unix_timestamp;

        Ok(())
    }

    pub fn init_tier(
        ctx : Context<InitTier>,
        _bump : u8,
        _seed :  String,
        _share : u32,
        _points : u8,
        _tokens : u64,
        ) -> ProgramResult {

        msg!("+ Init tier");

        let tier = &mut ctx.accounts.tier;

        tier.owner = *ctx.accounts.owner.key;
        tier.pool = ctx.accounts.pool.key();
        tier.share = _share;
        tier.points = _points;
        tier.tokens = _tokens;
        tier.count = 0;
        tier.royalty = 0;
        tier.claim_start = 0;
        tier.seed = _seed;
        tier.bump = _bump;

        Ok(())
    }
    
    pub fn create_clan(
        ctx : Context<CreateClan>,
        _bump : u8,
        _mints : Vec<Pubkey>
    ) -> ProgramResult {
        msg!("+ create clan");

        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        let clan_data = &mut ctx.accounts.clan_data;
        let tier = &mut ctx.accounts.tier;

        clan_data.owner = *ctx.accounts.owner.key;
        clan_data.tier = tier.key();
        clan_data.rand = *ctx.accounts.rand.key;
        clan_data.active = true;
        clan_data.create_time = clock.unix_timestamp;
        clan_data.last_claim_time = 0;
        clan_data.nfts = _mints;
        clan_data.bump = _bump;

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_reward_account.to_account_info().clone(),
            to: ctx.accounts.pool_reward_account.to_account_info().clone(),
            authority: ctx.accounts.owner.to_account_info().clone(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(token_cpi_ctx, tier.tokens)?;

        tier.count = tier.count + 1;

        Ok(())
    }

    pub fn create_company(
        ctx : Context<CreateCompany>,
        _bump : u8,
        _mints : Vec<Pubkey>
    ) -> ProgramResult {
        msg!("+ create company");

        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        let company_data = &mut ctx.accounts.company_data;
        let tier = &mut ctx.accounts.tier;

        company_data.owner = *ctx.accounts.owner.key;
        company_data.tier = tier.key();
        company_data.rand = *ctx.accounts.rand.key;
        company_data.active = true;
        company_data.create_time = clock.unix_timestamp;
        company_data.last_claim_time = 0;
        company_data.nfts = _mints;
        company_data.bump = _bump;

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_reward_account.to_account_info().clone(),
            to: ctx.accounts.pool_reward_account.to_account_info().clone(),
            authority: ctx.accounts.owner.to_account_info().clone(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(token_cpi_ctx, tier.tokens)?;

        tier.count = tier.count + 1;

        Ok(())
    }

    pub fn create_warparty(
        ctx : Context<CreateWarparty>,
        _bump : u8,
        _mints : Vec<Pubkey>
    ) -> ProgramResult {
        msg!("+ create warparty");

        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        let warparty_data = &mut ctx.accounts.warparty_data;
        let tier = &mut ctx.accounts.tier;

        warparty_data.owner = *ctx.accounts.owner.key;
        warparty_data.tier = tier.key();
        warparty_data.rand = *ctx.accounts.rand.key;
        warparty_data.active = true;
        warparty_data.create_time = clock.unix_timestamp;
        warparty_data.last_claim_time = 0;
        warparty_data.nfts = _mints;
        warparty_data.bump = _bump;

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_reward_account.to_account_info().clone(),
            to: ctx.accounts.pool_reward_account.to_account_info().clone(),
            authority: ctx.accounts.owner.to_account_info().clone(),
        };

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(token_cpi_ctx, tier.tokens)?;

        tier.count = tier.count + 1;

        Ok(())
    }

    pub fn remove_clan(
        ctx : Context<RemoveClan>,
    ) -> ProgramResult {
        msg!("+ remove clan");

        let clan_data = &mut ctx.accounts.clan_data;
        let tier_clan = &mut ctx.accounts.tier_clan;
        
        clan_data.active = false;
        tier_clan.count = tier_clan.count - 1;

        Ok(())
    }

    pub fn remove_company(
        ctx : Context<RemoveCompany>,
    ) -> ProgramResult {
        msg!("+ remove company");

        let company_data = &mut ctx.accounts.company_data;
        let tier_company = &mut ctx.accounts.tier_company;

        company_data.active = false;
        tier_company.count = tier_company.count - 1;

        Ok(())
    }

    pub fn remove_warparty(
        ctx : Context<RemoveWarparty>,
    ) -> ProgramResult {
        msg!("+ remove warparty");

        let warparty_data = &mut ctx.accounts.warparty_data;
        let tier_warparty = &mut ctx.accounts.tier_warparty;

        warparty_data.active = false;
        tier_warparty.count = tier_warparty.count - 1;

        Ok(())
    }

    pub fn claim_clan(
        ctx : Context<ClaimClan>
    ) -> ProgramResult {
        msg!("+ claim clan");

        let pool = &mut ctx.accounts.pool;
        let tier_clan = &ctx.accounts.tier_clan;
        let clan_data = &mut ctx.accounts.clan_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        
        if clan_data.active == false {
            return Err(PoolError::InvalidTier.into());
        }

        if (clan_data.last_claim_time as u64 > tier_clan.claim_start) || (tier_clan.claim_start > clock.unix_timestamp as u64) {
            return Err(PoolError::InvalidTime.into());
        }

        let clan_amount = tier_clan.royalty * tier_clan.share as u64 / tier_clan.count / 10000;

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_reward_account.to_account_info().clone(),
            to: ctx.accounts.user_reward_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(token_cpi_ctx, clan_amount)?;

        clan_data.last_claim_time = clock.unix_timestamp;

        Ok(())
    }

    pub fn claim_company(
        ctx : Context<ClaimCompany>
    ) -> ProgramResult {
        msg!("+ claim company");

        let pool = &mut ctx.accounts.pool;
        let tier_company = &ctx.accounts.tier_company;
        let company_data = &mut ctx.accounts.company_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        
        if company_data.active == false {
            return Err(PoolError::InvalidTier.into());
        }

        if (company_data.last_claim_time as u64 > tier_company.claim_start) || (tier_company.claim_start > clock.unix_timestamp as u64) {
            return Err(PoolError::InvalidTime.into());
        }

        let clan_amount = tier_company.royalty * tier_company.share as u64 / tier_company.count / 10000;

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_reward_account.to_account_info().clone(),
            to: ctx.accounts.user_reward_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(token_cpi_ctx, clan_amount)?;

        company_data.last_claim_time = clock.unix_timestamp;

        Ok(())
    }

    pub fn claim_warparty(
        ctx : Context<ClaimWarparty>
    ) -> ProgramResult {
        msg!("+ claim warparty");

        let pool = &mut ctx.accounts.pool;
        let tier_warparty = &ctx.accounts.tier_warparty;
        let warparty_data = &mut ctx.accounts.warparty_data;
        let clock = Clock::from_account_info(&ctx.accounts.clock)?;
        
        if warparty_data.active == false {
            return Err(PoolError::InvalidTier.into());
        }

        if (warparty_data.last_claim_time as u64 > tier_warparty.claim_start) || (tier_warparty.claim_start > clock.unix_timestamp as u64) {
            return Err(PoolError::InvalidTime.into());
        }

        let clan_amount = tier_warparty.royalty * tier_warparty.share as u64 / tier_warparty.count / 10000;

        let cpi_accounts = Transfer {
            from: ctx.accounts.pool_reward_account.to_account_info().clone(),
            to: ctx.accounts.user_reward_account.to_account_info().clone(),
            authority: pool.to_account_info().clone(),
        };

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let signer = &[&signer_seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info().clone();

        let token_cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(token_cpi_ctx, clan_amount)?;

        warparty_data.last_claim_time = clock.unix_timestamp;

        Ok(())
    }

    pub fn set_royalty(
        ctx : Context<SetRoyalty>,
        _royalty : u64,
        _start_time : u64
    ) -> ProgramResult {
        msg!("+ set royalty");

        let tier_clan = &mut ctx.accounts.tier_clan;
        let tier_company = &mut ctx.accounts.tier_company;
        let tier_warparty = &mut ctx.accounts.tier_warparty;

        tier_clan.royalty = _royalty;
        tier_clan.claim_start = _start_time;

        tier_company.royalty = _royalty;
        tier_company.claim_start = _start_time;

        tier_warparty.royalty = _royalty;
        tier_warparty.claim_start = _start_time;

        Ok(())
    }

    pub fn claim_solana(
        ctx : Context<ClaimSolana>,
        amount : u64
    ) -> ProgramResult {
        msg!("+ claim solana");

        let pool = &mut ctx.accounts.pool;

        let signer_seeds = &[
            pool.rand.as_ref(),
            &[pool.bump],
        ];

        let instruction = transfer(
            &pool.key(),
            &ctx.accounts.owner.key(), 
            amount
        );

        invoke_signed(
            &instruction, 
            &[
                pool.to_account_info().clone(),
                ctx.accounts.owner.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info().clone()
            ], 
            &[signer_seeds]
        );

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitPool<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(init, 
        seeds=[(*rand.key).as_ref()], 
        bump=_bump, 
        payer=owner, 
        space=8+POOL_SIZE)]
    pool : ProgramAccount<'info, Pool>,

    rand : AccountInfo<'info>,

    #[account(owner=spl_token::id())]
    reward_mint : Account<'info, Mint>,

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
pub struct UpdatePool<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    new_owner : AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(owner=spl_token::id())]
    reward_mint : Account<'info, Mint>,
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitCollection<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    creator : AccountInfo<'info>,

    #[account(init,
        seeds=[pool.key().as_ref(), (*creator.key).as_ref()],
        bump=_bump,
        payer=owner,
        space=8+COLLECTION_SIZE)]
    collection_data : ProgramAccount<'info,Collection>,

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
pub struct UpdateCollection<'info> {
    #[account(mut, signer)]
    owner : AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,
        has_one = owner,
        has_one = pool,
        seeds=[collection_data.pool.as_ref(), collection_data.creator.as_ref()],
        bump=collection_data.bump)]
    collection_data : ProgramAccount<'info,Collection>,

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct InitStakeData<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,owner=spl_token::id())]
    nft_mint : Account<'info, Mint>,

    #[account(init, 
        seeds=[nft_mint.key().as_ref(), owner.key().as_ref(), pool.key().as_ref()], 
        bump=_bump, 
        payer=owner, 
        space=8+STAKE_DATA_SIZE)]
    stake_data : ProgramAccount<'info,StakeData>,

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(
        seeds=[collection_data.pool.as_ref(), collection_data.creator.as_ref()],
        bump=collection_data.bump)]
    collection_data : ProgramAccount<'info,Collection>,

    #[account(mut,
        has_one = owner,
        has_one = pool,
        seeds=[stake_data.mint.as_ref(), stake_data.owner.as_ref(), stake_data.pool.as_ref()], 
        bump=stake_data.bump)]
    stake_data : ProgramAccount<'info,StakeData>,

    #[account(mut,owner=spl_token::id())]
    nft_mint : Account<'info, Mint>,

    #[account(mut)]
    metadata : AccountInfo<'info>,

    #[account(mut,
        constraint = user_nft_account.owner == owner.key(),
        constraint = user_nft_account.mint == nft_mint.key())]
    user_nft_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_nft_account.owner == pool.key(),
        constraint = pool_nft_account.mint == nft_mint.key())]
    pool_nft_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,    
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(
        seeds=[collection_data.pool.as_ref(), collection_data.creator.as_ref()],
        bump=collection_data.bump)]
    collection_data : ProgramAccount<'info,Collection>,

    #[account(mut,
        has_one = owner,
        has_one = pool,
        seeds=[stake_data.mint.as_ref(), stake_data.owner.as_ref(), stake_data.pool.as_ref()], 
        bump=stake_data.bump)]
    stake_data : ProgramAccount<'info,StakeData>,

    #[account(mut,owner=spl_token::id())]
    nft_mint : Account<'info, Mint>,

    #[account(mut)]
    metadata : AccountInfo<'info>,

    #[account(mut,
        constraint = user_nft_account.owner == owner.key(),
        constraint = user_nft_account.mint == nft_mint.key())]
    user_nft_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_nft_account.owner == pool.key(),
        constraint = pool_nft_account.mint == nft_mint.key())]
    pool_nft_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key(),
        constraint = user_reward_account.mint == pool.reward_mint)]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key(),
        constraint = pool_reward_account.mint == pool.reward_mint)]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,    
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(
        seeds=[collection_data.pool.as_ref(), collection_data.creator.as_ref()],
        bump=collection_data.bump)]
    collection_data : ProgramAccount<'info,Collection>,

    #[account(mut)]
    metadata : AccountInfo<'info>,

    #[account(mut,
        has_one = owner,
        has_one = pool,
        seeds=[stake_data.mint.as_ref(), stake_data.owner.as_ref(), stake_data.pool.as_ref()], 
        bump=stake_data.bump)]
    stake_data : ProgramAccount<'info,StakeData>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key(),
        constraint = user_reward_account.mint == pool.reward_mint)]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key(),
        constraint = pool_reward_account.mint == pool.reward_mint)]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,  
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()],
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut, owner = spl_token::id())]
    nft_mint : Account<'info, Mint>,

    #[account(mut,
        constraint = user_nft_account.owner == owner.key(),
        constraint = user_nft_account.mint == nft_mint.key())]
    user_nft_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_nft_account.owner == owner.key(),
        constraint = pool_nft_account.mint == pool.key())]
    pool_nft_account : Account<'info, TokenAccount>,

    token_program: Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(_bump : u8, _seed : String)]
pub struct InitTier<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(init, 
        seeds=[pool.key().as_ref(), _seed.as_ref()], 
        bump=_bump, 
        payer=owner, 
        space=8+TIER_DATA_SIZE)]
    tier : ProgramAccount<'info, TierData>,

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct CreateClan<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,
        seeds = [tier.pool.as_ref(), tier.seed.as_ref()], 
        bump = tier.bump)]
    tier : ProgramAccount<'info, TierData>,

    rand : AccountInfo<'info>,

    #[account(init, 
        seeds=[owner.key().as_ref(), tier.key().as_ref(), rand.key().as_ref()], 
        bump=_bump, 
        payer=owner, 
        space=8+CLAN_DATA_SIZE)]
    clan_data : ProgramAccount<'info,ClanData>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key(),
        constraint = user_reward_account.mint == pool.reward_mint)]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key(),
        constraint = pool_reward_account.mint == pool.reward_mint)]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,  

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct CreateCompany<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,
        seeds = [tier.pool.as_ref(), tier.seed.as_ref()], 
        bump = tier.bump)]
    tier : ProgramAccount<'info, TierData>,

    rand : AccountInfo<'info>,

    #[account(init, 
        seeds=[owner.key().as_ref(), tier.key().as_ref(), rand.key().as_ref()], 
        bump=_bump, 
        payer=owner, 
        space=8+COMPANY_DATA_SIZE)]
    company_data : ProgramAccount<'info,CompanyData>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key(),
        constraint = user_reward_account.mint == pool.reward_mint)]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key(),
        constraint = pool_reward_account.mint == pool.reward_mint)]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,  

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
#[instruction(_bump : u8)]
pub struct CreateWarparty<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(mut,
        seeds = [tier.pool.as_ref(), tier.seed.as_ref()], 
        bump = tier.bump)]
    tier : ProgramAccount<'info, TierData>,

    rand : AccountInfo<'info>,

    #[account(init, 
        seeds=[owner.key().as_ref(), tier.key().as_ref(), rand.key().as_ref()], 
        bump=_bump, 
        payer=owner, 
        space=8+WARPARTY_DATA_SIZE)]
    warparty_data : ProgramAccount<'info,WarpartyData>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key(),
        constraint = user_reward_account.mint == pool.reward_mint)]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key(),
        constraint = pool_reward_account.mint == pool.reward_mint)]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,  

    system_program : Program<'info,System>,
}

#[derive(Accounts)]
pub struct RemoveClan<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        seeds = [tier_clan.pool.as_ref(), tier_clan.seed.as_ref()], 
        bump = tier_clan.bump)]
    tier_clan : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds=[clan_data.owner.as_ref(), clan_data.tier.as_ref(), clan_data.rand.as_ref()], 
        bump=clan_data.bump,)]
    clan_data : ProgramAccount<'info,ClanData>
}

#[derive(Accounts)]
pub struct RemoveCompany<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        seeds = [tier_company.pool.as_ref(), tier_company.seed.as_ref()], 
        bump = tier_company.bump)]
    tier_company : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds=[company_data.owner.as_ref(), company_data.tier.as_ref(), company_data.rand.as_ref()], 
        bump=company_data.bump,)]
    company_data : ProgramAccount<'info,CompanyData>
}

#[derive(Accounts)]
pub struct RemoveWarparty<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        seeds = [tier_warparty.pool.as_ref(), tier_warparty.seed.as_ref()], 
        bump = tier_warparty.bump)]
    tier_warparty : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds=[warparty_data.owner.as_ref(), warparty_data.tier.as_ref(), warparty_data.rand.as_ref()], 
        bump=warparty_data.bump,)]
    warparty_data : ProgramAccount<'info,WarpartyData>
}

#[derive(Accounts)]
pub struct ClaimClan<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(
        seeds = [tier_clan.pool.as_ref(), tier_clan.seed.as_ref()], 
        bump = tier_clan.bump)]
    tier_clan : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds=[clan_data.owner.as_ref(), clan_data.tier.as_ref(), clan_data.rand.as_ref()], 
        bump=clan_data.bump,)]
    clan_data : ProgramAccount<'info,ClanData>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key())]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key())]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ClaimCompany<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(
        seeds = [tier_company.pool.as_ref(), tier_company.seed.as_ref()], 
        bump = tier_company.bump)]
    tier_company : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds=[company_data.owner.as_ref(), company_data.tier.as_ref(), company_data.rand.as_ref()], 
        bump=company_data.bump,)]
    company_data : ProgramAccount<'info,CompanyData>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key())]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key())]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ClaimWarparty<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    #[account(
        seeds = [tier_warparty.pool.as_ref(), tier_warparty.seed.as_ref()], 
        bump = tier_warparty.bump)]
    tier_warparty : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds=[warparty_data.owner.as_ref(), warparty_data.tier.as_ref(), warparty_data.rand.as_ref()], 
        bump=warparty_data.bump,)]
    warparty_data : ProgramAccount<'info,WarpartyData>,

    #[account(mut,
        constraint = user_reward_account.owner == owner.key())]
    user_reward_account : Account<'info, TokenAccount>,

    #[account(mut,
        constraint = pool_reward_account.owner == pool.key())]
    pool_reward_account : Account<'info, TokenAccount>,

    token_program:Program<'info, Token>,

    clock : AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SetRoyalty<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(mut,
        has_one = owner,
        seeds = [tier_clan.pool.as_ref(), tier_clan.seed.as_ref()], 
        bump = tier_clan.bump)]
    tier_clan : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds = [tier_company.pool.as_ref(), tier_company.seed.as_ref()], 
        bump = tier_company.bump)]
    tier_company : ProgramAccount<'info, TierData>,

    #[account(mut,
        has_one = owner,
        seeds = [tier_warparty.pool.as_ref(), tier_warparty.seed.as_ref()], 
        bump = tier_warparty.bump)]
    tier_warparty : ProgramAccount<'info, TierData>
}

#[derive(Accounts)]
pub struct ClaimSolana<'info> {
    #[account(mut)]
    owner : Signer<'info>,

    #[account(
        seeds = [pool.rand.as_ref()], 
        bump = pool.bump)]
    pool : ProgramAccount<'info, Pool>,

    system_program : Program<'info,System>
}

pub const POOL_SIZE : usize = 32 + 32 + 32 + 8 + 8 + 1 + 1;
pub const COLLECTION_SIZE : usize = 32 + 1 + 32 + 8 + 8 + 8 + 8 + 32 + 4 + 32 * 20;
pub const TIER_DATA_SIZE : usize = 32 + 1 + 32 + 4 + 1 + 8 + 8 + 8 + 8 + 4 + MAX_SYMBOL_LENGTH;
pub const STAKE_DATA_SIZE : usize = 1 + 1 + 8 + 32 + 32 + 32 + 8 + 8 + 1;
pub const CLAN_DATA_SIZE : usize = 32 + 1 + 32 + 32 + 1 + 8 + 8 + 4 + 32 * 7;
pub const COMPANY_DATA_SIZE : usize = 32 + 1 + 32 + 32  + 1 + 8 + 8 + 4 + 32 * 15;
pub const WARPARTY_DATA_SIZE : usize = 32 + 1 + 32 + 32  + 1 + 8 + 8 + 4 + 32 * 35;
pub const PERIOD : i64 = 24 * 60 * 60;

#[account]
pub struct Pool {
    pub owner : Pubkey,
    pub rand : Pubkey,
    pub reward_mint : Pubkey,
    pub start_time : i64,
    pub period : i64,
    pub tvl : u8,
    pub bump : u8,
}

#[account]
pub struct Collection {
    pub owner : Pubkey,
    pub bump : u8,
    pub pool : Pubkey,
    pub reward_normal : u64,
    pub reward_locked_one : u64,
    pub reward_locked_two : u64,
    pub reward_locked_three : u64,
    pub creator : Pubkey,
    pub ultras : Vec<Pubkey>
}

#[account]
pub struct TierData {
    pub owner : Pubkey,
    pub bump : u8,
    pub pool : Pubkey,
    pub share : u32,
    pub points : u8,
    pub tokens : u64,
    pub count : u64,
    pub royalty : u64,
    pub claim_start : u64,
    pub seed : String,
}

#[account]
pub struct StakeData {
    pub unstaked : bool,
    pub locked : bool,
    pub lock_period : u64,
    pub mint : Pubkey,
    pub owner : Pubkey,
    pub pool : Pubkey,
    pub stake_time : i64,
    pub last_claim_time : i64,
    pub bump : u8
}

#[account]
pub struct ClanData {
    pub owner : Pubkey,
    pub bump : u8,
    pub tier : Pubkey,
    pub rand : Pubkey,
    pub active : bool,
    pub create_time : i64,
    pub last_claim_time : i64,
    pub nfts : Vec<Pubkey>
}

#[account]
pub struct CompanyData {
    pub owner : Pubkey,
    pub bump : u8,
    pub tier : Pubkey,
    pub rand : Pubkey,
    pub active : bool,
    pub create_time : i64,
    pub last_claim_time : i64,
    pub nfts : Vec<Pubkey>
}

#[account]
pub struct WarpartyData {
    pub owner : Pubkey,
    pub bump : u8,
    pub tier : Pubkey,
    pub rand : Pubkey,
    pub active : bool,
    pub create_time : i64,
    pub last_claim_time : i64,
    pub nfts : Vec<Pubkey>
}

#[error]
pub enum PoolError {
    #[msg("Invalid tier")]
    InvalidTier,

    #[msg("Token set authority failed")]
    TokenSetAuthorityFailed,

    #[msg("Token transfer failed")]
    TokenTransferFailed,

    #[msg("Invalid token account")]
    InvalidTokenAccount,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Invalid metadata")]
    InvalidMetadata,

    #[msg("Invalid stakedata account")]
    InvalidStakeData,

    #[msg("Invalid time")]
    InvalidTime,

    #[msg("Invalid Period")]
    InvalidPeriod,

    #[msg("Already unstaked")]
    AlreadyUnstaked,

    #[msg("Invalid owner")]
    InvalidOwner
}