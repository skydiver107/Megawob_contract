use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Mint, Token};
use anchor_lang::solana_program::{pubkey::Pubkey, entrypoint::ProgramResult, clock::Clock, program_error::ProgramError};
use std::convert::Into;
use crate::constants::*;
mod constants {
    use solana_program::pubkey::Pubkey;
    pub const DAY_TIME: u32 = 86400;//devnet 60, mainnet 86400
    pub const DECIMAL: u64 = 1000000000;
    pub const REWARD: [u8;3] = [5, 10, 17];
    pub const PERIOD: [u8;3] = [7, 14, 21];
    pub const EXTRA_REWARD: u8 = 2;
    pub const CANDYMACHINE_ID1: Pubkey = anchor_lang::solana_program::pubkey!("6EYg3NCmywPrhgNBG43Sxunpnkb6vvhDoaEbtX22AC2d");
    pub const CANDYMACHINE_ID2: Pubkey = anchor_lang::solana_program::pubkey!("6EYg3NCmywPrhgNBG43Sxunpnkb6vvhDoaEbtX22AC2d");
}
declare_id!("82P6ZBwhkBvYjAG7pQPnEfKWSPJYByeWGjTVha5VKWE");//5tvBzSCacUn2maQkeaU7TzNX7rspSpEnxHagzZceaP7G on devnet

#[program]
pub mod megawob_contract {
    use super::*;

    pub fn create_vault(_ctx: Context<CreateVault>, _bump: u8) -> ProgramResult {
        Ok(())
    }
    pub fn create_data(_ctx: Context<CreateData>, _bump: u8) -> ProgramResult {
        let data = &mut _ctx.accounts.data;
        data.total_staked_count = 0;
        data.reward = 0;
        Ok(())
    }
    pub fn create_poolsigner(_ctx: Context<CreatePoolSigner>, _bump: u8) -> ProgramResult {
        Ok(())
    }
    pub fn create_pool(_ctx:Context<CreatePool>, _bump: u8) -> ProgramResult {
        let pool = &mut _ctx.accounts.pool;
        pool.user = _ctx.accounts.user.key();
        pool.mint = _ctx.accounts.mint.key();
        pool.is_staked = false;
        pool.claimed_amount = 0;

        Ok(())
    }
    pub fn stake(_ctx:Context<StakeContext>, _type: u8) -> ProgramResult {
        let pool = &mut _ctx.accounts.pool;
        let data = &mut _ctx.accounts.data;
        let m_data = &mut _ctx.accounts.metadata.try_borrow_data()?;
        let metadata = mpl_token_metadata::state::Metadata::deserialize(&mut &m_data[..])?;
        let collection_not_proper = metadata
            .data
            .creators
            .as_ref()
            .unwrap()
            .iter()
            .filter(|item| {
                (item.address == CANDYMACHINE_ID1 || item.address == CANDYMACHINE_ID2) && item.verified
            })
            .count() == 0;
        if collection_not_proper || metadata.mint != _ctx.accounts.mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        let clock = Clock::get().unwrap();
        if pool.claimed_amount > 0 {
            if clock.unix_timestamp as u32 - pool.end_time < DAY_TIME * 2 {
                return Err(ProgramError::InvalidArgument);//
            }
        }
        let cpi_ctx = CpiContext::new(
            _ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: _ctx.accounts.nft_from.to_account_info(),
                to: _ctx.accounts.nft_to.to_account_info(),
                authority: _ctx.accounts.user.to_account_info()
            }
        );
        token::transfer(cpi_ctx, 1)?;

        data.total_staked_count += 1;
        pool.start_time = clock.unix_timestamp as u32;
        pool.last_update_time = clock.unix_timestamp as u32;
        pool.pool_type = _type;
        pool.end_time = pool.start_time + PERIOD[_type as usize] as u32 * DAY_TIME;
        pool.is_staked = true;
        Ok(())
    }
    pub fn claim(_ctx: Context<ClaimContext>, bump: u8) -> ProgramResult {
        let pool = &mut _ctx.accounts.pool;
        let data = &mut _ctx.accounts.data;
        let m_data = &mut _ctx.accounts.metadata.try_borrow_data()?;
        let metadata = mpl_token_metadata::state::Metadata::deserialize(&mut &m_data[..])?;
        let collection_not_proper = metadata
            .data
            .creators
            .as_ref()
            .unwrap()
            .iter()
            .filter(|item| {
                (item.address == CANDYMACHINE_ID1 || item.address == CANDYMACHINE_ID2) && item.verified
            })
            .count() == 0;
        if collection_not_proper {
            return Err(ProgramError::InvalidAccountData);
        }
        let clock = Clock::get().unwrap();
        let cur_time: u32;
        if clock.unix_timestamp as u32 > pool.end_time {
            cur_time = pool.end_time;
        } else {
            cur_time = clock.unix_timestamp as u32;
        }
        let life_time = cur_time - pool.last_update_time;
        let index = pool.pool_type as usize;
        let transfer_amount = DECIMAL * life_time as u64 / DAY_TIME as u64 * REWARD[index] as u64;
        let seeds = &[b"furrsol vault".as_ref(), &[bump]];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            _ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: _ctx.accounts.token_from.to_account_info(),
                to: _ctx.accounts.token_to.to_account_info(),
                authority: _ctx.accounts.vault.to_account_info(),
            },
            signer
        );
        token::transfer(cpi_ctx, transfer_amount.into())?;
        data.reward += transfer_amount;
        pool.claimed_amount += transfer_amount;
        pool.last_update_time = cur_time;
        Ok(())
    }
    pub fn unstake(_ctx: Context<UnstakeContext>, bump_vault: u8, bump_signer: u8) -> ProgramResult {
        let pool = &mut _ctx.accounts.pool;
        let data = &mut _ctx.accounts.data;
        let m_data = &mut _ctx.accounts.metadata.try_borrow_data()?;
        let metadata = mpl_token_metadata::state::Metadata::deserialize(&mut &m_data[..])?;
        let collection_not_proper = metadata
            .data
            .creators
            .as_ref()
            .unwrap()
            .iter()
            .filter(|item| {
                (item.address == CANDYMACHINE_ID1 || item.address == CANDYMACHINE_ID2) && item.verified
            })
            .count() == 0;
        if collection_not_proper || metadata.mint != _ctx.accounts.mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }
        let clock = Clock::get().unwrap();
        let cur_time = clock.unix_timestamp as u32;
        if pool.end_time > cur_time {
            return Err(ProgramError::InvalidArgument);
        }
        if pool.last_update_time > pool.end_time {
            return Err(ProgramError::InvalidInstructionData);
        }

        let life_time = pool.end_time - pool.last_update_time;
        let index = pool.pool_type as usize;
        let transfer_amount = DECIMAL * life_time as u64 / DAY_TIME as u64 * REWARD[index] as u64;
        let seeds = &[b"furrsol vault".as_ref(), &[bump_vault]];
        let signer = &[&seeds[..]];
        let mut cpi_ctx = CpiContext::new_with_signer(
            _ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: _ctx.accounts.token_from.to_account_info(),
                to: _ctx.accounts.token_to.to_account_info(),
                authority: _ctx.accounts.vault.to_account_info(),
            },
            signer
        );
        token::transfer(cpi_ctx, transfer_amount.into())?;
        let pool_seeds = &[b"furrsol signer".as_ref(), _ctx.accounts.user.to_account_info().key.as_ref(),  &[bump_signer]];
        let pool_signer = &[&pool_seeds[..]];
        cpi_ctx = CpiContext::new_with_signer(_ctx.accounts.token_program.to_account_info(), token::Transfer {
            from: _ctx.accounts.nft_from.to_account_info(),
            to: _ctx.accounts.nft_to.to_account_info(),
            authority: _ctx.accounts.pool_signer.to_account_info()
        }, pool_signer);
        token::transfer(cpi_ctx, 1)?;

        data.total_staked_count -= 1;
        data.reward += transfer_amount;
        pool.claimed_amount += transfer_amount;
        pool.is_staked = false;
        pool.end_time = cur_time;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateVault<'info> {
    #[account(init, seeds = [b"furrsol vault".as_ref()], bump, payer = admin, space = 8 + 1)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreateData<'info> {
    #[account(init, seeds = [b"furrsol data".as_ref()], bump, payer = admin, space = 8 + 2 + 8 )]
    pub data: Account<'info, Data>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreatePoolSigner<'info> {
    #[account(init, seeds = [b"furrsol signer".as_ref(), user.key().as_ref()], bump, payer = user, space = 8 + 1)]
    pub pool_signer: Account<'info, PoolSigner>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct CreatePool<'info> {
    #[account(init, seeds = [b"furrsol pool".as_ref(), user.key().as_ref(), mint.key().as_ref()], bump,  payer = user, space = 8 + 69 + 4 + 5 + 8)]
    pub pool: Account<'info, Pool>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct StakeContext<'info> {
    pub user: Signer<'info>,
    #[account(mut, has_one = mint, has_one = user)]
    pub pool: Account<'info, Pool>,
    #[account(mut)]
    pub nft_from: Account<'info, TokenAccount>,
    #[account(mut)]
    pub nft_to: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub data: Account<'info, Data>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub metadata: AccountInfo<'info>,
    pub token_program: Program<'info, Token>
}
#[derive(Accounts)]
pub struct ClaimContext<'info> {
    pub user: Signer<'info>,
    #[account(mut, has_one = user)]
    pub pool: Account<'info, Pool>,
    #[account(mut)]
    pub data: Account<'info, Data>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub metadata: AccountInfo<'info>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub token_from: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_to: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>
}
#[derive(Accounts)]
pub struct UnstakeContext<'info> {
    pub user: Signer<'info>,
    pub pool_signer: Account<'info, PoolSigner>,
    #[account(mut, has_one = user)]
    pub pool: Account<'info, Pool>,
    #[account(mut)]
    pub data: Account<'info, Data>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub metadata: AccountInfo<'info>,
    #[account(mut)]
    pub nft_from: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub nft_to: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_from: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub token_to: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>
}


#[account]
pub struct Vault {
    pub bump_vault: u8
}
#[account]
pub struct Data {
    pub total_staked_count: u16,
    pub reward: u64,

}
#[account]
pub struct PoolSigner {
    pub bump_signer: u8
}

#[account]
pub struct Pool {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub start_time: u32,
    pub is_staked: bool,
    pub last_update_time: u32,
    pub end_time: u32,
    pub pool_type: u8,
    pub claimed_amount: u64
}