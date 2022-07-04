pub mod error;

use std::io::Write;
use std::ops::Deref;
use std::str::FromStr;

use crate::error::PortAdaptorError;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Slot;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::solana_program::program_error::ProgramError as Error;
use anchor_lang::solana_program::program_option::COption;
use anchor_lang::solana_program::program_pack::Pack;
use port_staking_instructions::instruction::{
    claim_reward as port_claim_reward, create_stake_account as create_port_stake_account,
    deposit as port_staking_deposit, init_staking_pool as init_port_staking_pool,
    withdraw as port_staking_withdraw,
};
use port_staking_instructions::state::{StakeAccount, StakingPool};
use port_variable_rate_lending_instructions::instruction::{
    borrow_obligation_liquidity, deposit_reserve_liquidity,
    deposit_reserve_liquidity_and_obligation_collateral, redeem_reserve_collateral,
    refresh_obligation, refresh_reserve, repay_obligation_liquidity,
    withdraw_obligation_collateral, LendingInstruction,
};
use port_variable_rate_lending_instructions::state::{
    CollateralExchangeRate, LendingMarket, Obligation, Reserve,
};

pub use port_staking_instructions::id as port_staking_id;
pub use port_variable_rate_lending_instructions::id;

#[cfg(feature = "devnet")]
pub fn port_lending_id() -> Pubkey {
    Pubkey::from_str("pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4").unwrap()
}

#[cfg(not(feature = "devnet"))]
pub fn port_lending_id() -> Pubkey {
    id()
}

pub fn init_obligation<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, InitObligation<'info>>,
) -> ProgramResult {
    let ix = Instruction {
        program_id: port_lending_id(),
        accounts: vec![
            AccountMeta::new(ctx.accounts.obligation.key(), false),
            AccountMeta::new_readonly(ctx.accounts.lending_market.key(), false),
            AccountMeta::new_readonly(ctx.accounts.obligation_owner.key(), true),
            AccountMeta::new_readonly(ctx.accounts.clock.key(), false),
            AccountMeta::new_readonly(ctx.accounts.rent.key(), false),
            AccountMeta::new_readonly(ctx.accounts.spl_token_id.key(), false),
        ],
        data: LendingInstruction::InitObligation.pack(),
    };

    invoke_signed(
        &ix,
        &[
            ctx.accounts.obligation,
            ctx.accounts.lending_market,
            ctx.accounts.obligation_owner,
            ctx.accounts.clock,
            ctx.accounts.rent,
            ctx.accounts.spl_token_id,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct InitObligation<'info> {
    pub obligation: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub obligation_owner: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
    pub spl_token_id: AccountInfo<'info>,
}

pub fn deposit_reserve<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Deposit<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = deposit_reserve_liquidity(
        port_lending_id(),
        amount,
        ctx.accounts.source_liquidity.key(),
        ctx.accounts.destination_collateral.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.reserve_liquidity_supply.key(),
        ctx.accounts.reserve_collateral_mint.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.transfer_authority.key(),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_liquidity,
            ctx.accounts.destination_collateral,
            ctx.accounts.reserve,
            ctx.accounts.reserve_liquidity_supply,
            ctx.accounts.reserve_collateral_mint,
            ctx.accounts.lending_market,
            ctx.accounts.lending_market_authority,
            ctx.accounts.transfer_authority,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub source_liquidity: AccountInfo<'info>,
    pub destination_collateral: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub reserve_liquidity_supply: AccountInfo<'info>,
    pub reserve_collateral_mint: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    pub transfer_authority: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

pub fn deposit_and_collateralize<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, DepositAndCollateralize<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = deposit_reserve_liquidity_and_obligation_collateral(
        port_lending_id(),
        amount,
        ctx.accounts.source_liquidity.key(),
        ctx.accounts.user_collateral.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.reserve_liquidity_supply.key(),
        ctx.accounts.reserve_collateral_mint.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.destination_collateral.key(),
        ctx.accounts.obligation.key(),
        ctx.accounts.obligation_owner.key(),
        ctx.accounts.transfer_authority.key(),
        Some(ctx.accounts.stake_account.key()),
        Some(ctx.accounts.staking_pool.key()),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_liquidity,
            ctx.accounts.user_collateral,
            ctx.accounts.reserve,
            ctx.accounts.reserve_liquidity_supply,
            ctx.accounts.reserve_collateral_mint,
            ctx.accounts.lending_market,
            ctx.accounts.lending_market_authority,
            ctx.accounts.destination_collateral,
            ctx.accounts.obligation,
            ctx.accounts.obligation_owner,
            ctx.accounts.transfer_authority,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.accounts.stake_account,
            ctx.accounts.staking_pool,
            ctx.accounts.port_staking_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct DepositAndCollateralize<'info> {
    pub source_liquidity: AccountInfo<'info>,
    pub user_collateral: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub reserve_liquidity_supply: AccountInfo<'info>,
    pub reserve_collateral_mint: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    pub destination_collateral: AccountInfo<'info>,
    pub obligation: AccountInfo<'info>,
    pub obligation_owner: AccountInfo<'info>,
    pub stake_account: AccountInfo<'info>,
    pub staking_pool: AccountInfo<'info>,
    pub transfer_authority: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub port_staking_program: AccountInfo<'info>,
}

pub fn borrow<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Borrow<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = borrow_obligation_liquidity(
        port_lending_id(),
        amount,
        ctx.accounts.source_liquidity.key(),
        ctx.accounts.destination_liquidity.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.reserve_fee_receiver.key(),
        ctx.accounts.obligation.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.obligation_owner.key(),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_liquidity,
            ctx.accounts.destination_liquidity,
            ctx.accounts.reserve,
            ctx.accounts.reserve_fee_receiver,
            ctx.accounts.obligation,
            ctx.accounts.lending_market,
            ctx.accounts.lending_market_authority,
            ctx.accounts.obligation_owner,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct Borrow<'info> {
    pub source_liquidity: AccountInfo<'info>,
    pub destination_liquidity: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub reserve_fee_receiver: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    pub obligation: AccountInfo<'info>,
    pub obligation_owner: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

pub fn repay<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Repay<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = repay_obligation_liquidity(
        port_lending_id(),
        amount,
        ctx.accounts.source_liquidity.key(),
        ctx.accounts.destination_liquidity.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.obligation.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.transfer_authority.key(),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_liquidity,
            ctx.accounts.destination_liquidity,
            ctx.accounts.reserve,
            ctx.accounts.obligation,
            ctx.accounts.lending_market,
            ctx.accounts.transfer_authority,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct Repay<'info> {
    pub source_liquidity: AccountInfo<'info>,
    pub destination_liquidity: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub obligation: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub transfer_authority: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

pub fn withdraw<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Withdraw<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = withdraw_obligation_collateral(
        port_lending_id(),
        amount,
        ctx.accounts.source_collateral.key(),
        ctx.accounts.destination_collateral.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.obligation.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.obligation_owner.key(),
        Some(ctx.accounts.stake_account.key()),
        Some(ctx.accounts.staking_pool.key()),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_collateral,
            ctx.accounts.destination_collateral,
            ctx.accounts.reserve,
            ctx.accounts.obligation,
            ctx.accounts.lending_market,
            ctx.accounts.lending_market_authority,
            ctx.accounts.obligation_owner,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.accounts.stake_account,
            ctx.accounts.staking_pool,
            ctx.accounts.port_staking_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub source_collateral: AccountInfo<'info>,
    pub destination_collateral: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub obligation: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    pub stake_account: AccountInfo<'info>,
    pub staking_pool: AccountInfo<'info>,
    pub obligation_owner: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub port_staking_program: AccountInfo<'info>,
}

pub fn redeem<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, Redeem<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = redeem_reserve_collateral(
        port_lending_id(),
        amount,
        ctx.accounts.source_collateral.key(),
        ctx.accounts.destination_liquidity.key(),
        ctx.accounts.reserve.key(),
        ctx.accounts.reserve_collateral_mint.key(),
        ctx.accounts.reserve_liquidity_supply.key(),
        ctx.accounts.lending_market.key(),
        ctx.accounts.transfer_authority.key(),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.source_collateral,
            ctx.accounts.destination_liquidity,
            ctx.accounts.reserve,
            ctx.accounts.reserve_collateral_mint,
            ctx.accounts.reserve_liquidity_supply,
            ctx.accounts.lending_market,
            ctx.accounts.lending_market_authority,
            ctx.accounts.transfer_authority,
            ctx.accounts.clock,
            ctx.accounts.token_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct Redeem<'info> {
    pub source_collateral: AccountInfo<'info>,
    pub destination_liquidity: AccountInfo<'info>,
    pub reserve: AccountInfo<'info>,
    pub reserve_collateral_mint: AccountInfo<'info>,
    pub reserve_liquidity_supply: AccountInfo<'info>,
    pub lending_market: AccountInfo<'info>,
    pub lending_market_authority: AccountInfo<'info>,
    pub transfer_authority: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
}

pub fn refresh_port_reserve<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, RefreshReserve<'info>>,
) -> ProgramResult {
    let oracle = ctx.remaining_accounts;
    let ix = refresh_reserve(
        port_lending_id(),
        ctx.accounts.reserve.key(),
        oracle
            .first()
            .map_or(COption::None, |k| COption::Some(k.key())),
    );
    let mut accounts = vec![ctx.accounts.reserve, ctx.accounts.clock, ctx.program];
    accounts.extend(oracle.into_iter().next());
    invoke(&ix, &accounts).map_err(Into::into)
}

#[derive(Accounts)]
pub struct RefreshReserve<'info> {
    pub reserve: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
}

pub fn refresh_port_obligation<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, RefreshObligation<'info>>,
) -> ProgramResult {
    let reserves = ctx.remaining_accounts;
    let ix = refresh_obligation(
        port_lending_id(),
        ctx.accounts.obligation.key(),
        reserves.iter().map(|info| info.key()).collect(),
    );
    let mut account_infos = vec![ctx.accounts.obligation, ctx.accounts.clock];
    account_infos.extend(reserves);
    account_infos.push(ctx.program);
    invoke(&ix, &account_infos).map_err(Into::into)
}

#[derive(Accounts)]
pub struct RefreshObligation<'info> {
    pub obligation: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
}

pub fn claim_reward<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, ClaimReward<'info>>,
    sub_reward_pool: Option<AccountInfo<'info>>,
    sub_reward_dest: Option<AccountInfo<'info>>,
) -> ProgramResult {
    let ix = port_claim_reward(
        port_staking_id(),
        ctx.accounts.stake_account_owner.key(),
        ctx.accounts.stake_account.key(),
        ctx.accounts.staking_pool.key(),
        ctx.accounts.reward_token_pool.key(),
        ctx.accounts.reward_dest.key(),
        sub_reward_pool.as_ref().map_or(None, |v| Some(v.key())),
        sub_reward_dest.as_ref().map_or(None, |v| Some(v.key())),
    );

    let accounts : Vec<AccountInfo<'info>> = vec![
        ctx.accounts.stake_account_owner,
        ctx.accounts.stake_account,
        ctx.accounts.staking_pool,
        ctx.accounts.reward_token_pool,
        ctx.accounts.reward_dest,
        ctx.accounts.staking_program_authority,
        ctx.accounts.clock,
        ctx.accounts.token_program,
    ]
    .into_iter()
    .chain(match [sub_reward_pool, sub_reward_dest] {
        [Some(pool), Some(reward)] => vec![pool, reward],
        _ => vec![],
    })
    .chain([ctx.program])
    .collect();

    invoke_signed(
        &ix,
        &accounts[..],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts, Clone)]
pub struct ClaimReward<'info> {
    pub stake_account_owner: AccountInfo<'info>,
    pub stake_account: AccountInfo<'info>,
    pub staking_pool: AccountInfo<'info>,
    pub reward_token_pool: AccountInfo<'info>,
    pub reward_dest: AccountInfo<'info>,
    pub staking_program_authority: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

pub fn create_port_staking_pool<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, CreateStakingPoolContext<'info>>,
    supply: u64,
    duration: u64,
    earliest_reward_claim_time: Slot,
) -> ProgramResult {
    let ix = init_port_staking_pool(
        port_staking_id(),
        supply,
        duration,
        earliest_reward_claim_time,
        ctx.accounts.transfer_authority.key(),
        ctx.accounts.reward_token_supply.key(),
        ctx.accounts.reward_token_pool.key(),
        ctx.accounts.staking_pool.key(),
        ctx.accounts.reward_token_mint.key(),
        ctx.accounts.staking_pool_owner.key(),
        ctx.accounts.admin.key(),
    );

    invoke_signed(
        &ix,
        &[
            ctx.accounts.transfer_authority,
            ctx.accounts.reward_token_supply,
            ctx.accounts.reward_token_pool,
            ctx.accounts.staking_pool,
            ctx.accounts.reward_token_mint,
            ctx.accounts.staking_program_derived,
            ctx.accounts.rent,
            ctx.accounts.token_program,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts, Clone)]
pub struct CreateStakingPoolContext<'info> {
    pub staking_pool: AccountInfo<'info>,
    pub transfer_authority: AccountInfo<'info>,
    pub reward_token_supply: AccountInfo<'info>,
    pub reward_token_pool: AccountInfo<'info>,
    pub reward_token_mint: AccountInfo<'info>,
    pub staking_pool_owner: AccountInfo<'info>,
    pub admin: AccountInfo<'info>,
    pub staking_program_derived: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

pub fn create_stake_account<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, CreateStakeAccount<'info>>,
) -> ProgramResult {
    let ix = create_port_stake_account(
        port_staking_id(),
        ctx.accounts.stake_account.key(),
        ctx.accounts.staking_pool.key(),
        ctx.accounts.owner.key(),
    );
    invoke_signed(
        &ix,
        &[
            ctx.accounts.stake_account,
            ctx.accounts.staking_pool,
            ctx.accounts.owner,
            ctx.accounts.rent,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts, Clone)]
pub struct CreateStakeAccount<'info> {
    pub staking_pool: AccountInfo<'info>,
    pub stake_account: AccountInfo<'info>,
    pub owner: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}

pub fn port_stake<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, PortStake<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = port_staking_deposit(
        port_staking_id(),
        amount,
        ctx.accounts.authority.key(),
        ctx.accounts.stake_account.key(),
        ctx.accounts.staking_pool.key(),
    );
    invoke_signed(
        &ix,
        &[
            ctx.accounts.stake_account,
            ctx.accounts.staking_pool,
            ctx.accounts.authority,
            ctx.accounts.clock,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts, Clone)]
pub struct PortStake<'info> {
    pub staking_pool: AccountInfo<'info>,
    pub stake_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
}

pub fn port_unstake<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, PortUnstake<'info>>,
    amount: u64,
) -> ProgramResult {
    let ix = port_staking_withdraw(
        port_staking_id(),
        amount,
        ctx.accounts.authority.key(),
        ctx.accounts.stake_account.key(),
        ctx.accounts.staking_pool.key(),
    );
    invoke_signed(
        &ix,
        &[
            ctx.accounts.stake_account,
            ctx.accounts.staking_pool,
            ctx.accounts.authority,
            ctx.accounts.clock,
            ctx.program,
        ],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts, Clone)]
pub struct PortUnstake<'info> {
    pub staking_pool: AccountInfo<'info>,
    pub stake_account: AccountInfo<'info>,
    pub authority: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
}

pub mod port_accessor {
    use std::convert::TryFrom;

    use anchor_lang::solana_program::program_error::ProgramError as Error;
    use anchor_lang::solana_program::pubkey::PUBKEY_BYTES;
    use port_variable_rate_lending_instructions::math::{Rate as PortRate, U128};
    use port_variable_rate_lending_instructions::state::{
        CollateralExchangeRate, INITIAL_COLLATERAL_RATE, OBLIGATION_COLLATERAL_LEN,
        OBLIGATION_LIQUIDITY_LEN,
    };

    use solana_maths::{Decimal, Rate, TryAdd, TryDiv, TrySub};

    use crate::error::PortAdaptorError;

    use super::*;

    fn unpack_decimal(src: &[u8; 16]) -> Decimal {
        Decimal::from_scaled_val(u128::from_le_bytes(*src))
    }

    pub fn reserve_ltv(account: &AccountInfo) -> std::result::Result<u8, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 1];
        amount_bytes.copy_from_slice(&bytes[304..305]);
        Ok(u8::from_le_bytes(amount_bytes))
    }

    pub fn reserve_available_liquidity(account: &AccountInfo) -> std::result::Result<u64, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[175..183]);
        Ok(u64::from_le_bytes(amount_bytes))
    }

    pub fn reserve_borrowed_amount(account: &AccountInfo) -> std::result::Result<Decimal, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 16];
        amount_bytes.copy_from_slice(&bytes[183..199]);
        Ok(unpack_decimal(&amount_bytes))
    }

    pub fn reserve_market_price(account: &AccountInfo) -> std::result::Result<Decimal, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 16];
        amount_bytes.copy_from_slice(&bytes[215..231]);
        Ok(unpack_decimal(&amount_bytes))
    }

    pub fn reserve_oracle_pubkey(account: &AccountInfo) -> std::result::Result<Pubkey, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 32];
        amount_bytes.copy_from_slice(&bytes[143..175]);
        Ok(Pubkey::new_from_array(amount_bytes))
    }

    pub fn reserve_total_liquidity(account: &AccountInfo) -> std::result::Result<Decimal, Error> {
        let available_liquidity = reserve_available_liquidity(account)?;
        let borrowed_amount = reserve_borrowed_amount(account)?;
        borrowed_amount
            .try_add(Decimal::from(available_liquidity))
            .map_err(Into::into)
    }

    pub fn reserve_liquidity_mint_pubkey(
        account: &AccountInfo,
    ) -> std::result::Result<Pubkey, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 32];
        amount_bytes.copy_from_slice(&bytes[42..74]);
        Ok(Pubkey::new_from_array(amount_bytes))
    }

    pub fn reserve_lp_mint_pubkey(account: &AccountInfo) -> std::result::Result<Pubkey, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 32];
        amount_bytes.copy_from_slice(&bytes[231..263]);
        Ok(Pubkey::new_from_array(amount_bytes))
    }

    pub fn reserve_mint_total(account: &AccountInfo) -> std::result::Result<u64, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[263..271]);
        Ok(u64::from_le_bytes(amount_bytes))
    }

    pub fn reserve_borrow_fee(account: &AccountInfo) -> std::result::Result<Rate, Error> {
        let bytes = account.try_borrow_data()?;
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&bytes[310..318]);
        Ok(Rate::from_scaled_val(u64::from_le_bytes(amount_bytes)))
    }

    pub fn exchange_rate(
        account: &AccountInfo,
    ) -> std::result::Result<CollateralExchangeRate, Error> {
        let mint_total_supply = reserve_mint_total(account)?;
        let total_liquidity = reserve_total_liquidity(account)?;
        let rate = if mint_total_supply == 0 || total_liquidity == Decimal::zero() {
            Rate::from_scaled_val(INITIAL_COLLATERAL_RATE)
        } else {
            let mint_total_supply = Decimal::from(mint_total_supply);
            Rate::try_from(mint_total_supply.try_div(total_liquidity)?)?
        };
        let port_rate = PortRate(U128::from(rate.to_scaled_val()));
        Ok(CollateralExchangeRate(port_rate))
    }

    pub fn obligation_deposits_count(account: &AccountInfo) -> std::result::Result<u8, Error> {
        let bytes = account.try_borrow_data()?;
        Ok(bytes[138])
    }

    pub fn obligation_borrows_count(account: &AccountInfo) -> std::result::Result<u8, Error> {
        let bytes = account.try_borrow_data()?;
        Ok(bytes[139])
    }

    pub fn obligation_borrow_amount_wads(
        account: &AccountInfo,
        n: u8,
    ) -> std::result::Result<Decimal, Error> {
        let bytes = account.try_borrow_data()?;
        let deposit_lens = obligation_deposits_count(account)?;
        let borrows_lens = obligation_borrows_count(account)?;
        if n >= borrows_lens {
            msg!("No enough borrows");
            return Err(PortAdaptorError::BorrowIndexOutOfBound.into());
        }
        let mut amount_bytes = [0u8; 16];
        let start_index = 140
            + (deposit_lens as usize) * OBLIGATION_COLLATERAL_LEN
            + n as usize * OBLIGATION_LIQUIDITY_LEN
            + PUBKEY_BYTES
            + 16;

        amount_bytes.copy_from_slice(&bytes[start_index..(start_index + 16)]);
        Ok(unpack_decimal(&amount_bytes))
    }

    pub fn obligation_deposit_amount(
        account: &AccountInfo,
        n: u8,
    ) -> std::result::Result<u64, Error> {
        let bytes = account.try_borrow_data()?;
        let deposit_lens = obligation_deposits_count(account)?;
        if n >= deposit_lens {
            msg!("No enough deposits");
            return Err(PortAdaptorError::CollateralIndexOutOfBound.into());
        }
        let mut amount_bytes = [0u8; 8];
        let start_index = 140 + n as usize * OBLIGATION_COLLATERAL_LEN + PUBKEY_BYTES;

        amount_bytes.copy_from_slice(&bytes[start_index..(start_index + 8)]);
        Ok(u64::from_le_bytes(amount_bytes))
    }
    pub fn obligation_liquidity(
        account: &AccountInfo,
        port_exchange_rate: &CollateralExchangeRate,
        deposit_index: u8,
        borrow_index: u8,
    ) -> std::result::Result<Decimal, Error> {
        let deposit = if obligation_deposits_count(account)? == 0 {
            0u64
        } else {
            port_exchange_rate
                .collateral_to_liquidity(obligation_deposit_amount(account, deposit_index)?)?
        };
        let borrow = if obligation_borrows_count(account)? == 0 {
            Decimal::zero()
        } else {
            obligation_borrow_amount_wads(account, borrow_index)?
        };
        Decimal::from(deposit).try_sub(borrow).map_err(Into::into)
    }

    pub fn is_obligation_stale(account: &AccountInfo) -> std::result::Result<bool, Error> {
        let bytes = account.try_borrow_data()?;
        Ok(bytes[9] == 1)
    }

    pub fn is_reserve_stale(account: &AccountInfo) -> std::result::Result<bool, Error> {
        let bytes = account.try_borrow_data()?;
        Ok(bytes[9] == 1)
    }
}
#[derive(Clone)]
pub struct PortStakeAccount(StakeAccount);

impl PortStakeAccount {
    pub const LEN: usize = StakeAccount::LEN;
}

impl anchor_lang::AccountDeserialize for PortStakeAccount {
    fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        PortStakeAccount::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        StakeAccount::unpack(buf)
            .map(PortStakeAccount)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for PortStakeAccount {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> std::result::Result<(), Error> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for PortStakeAccount {
    fn owner() -> Pubkey {
        port_staking_id()
    }
}

impl Deref for PortStakeAccount {
    type Target = StakeAccount;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct PortReserve(Reserve);

impl anchor_lang::AccountDeserialize for PortReserve {
    fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        PortReserve::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        Reserve::unpack(buf).map(PortReserve).map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for PortReserve {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> std::result::Result<(), Error> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for PortReserve {
    fn owner() -> Pubkey {
        port_lending_id()
    }
}

impl Deref for PortReserve {
    type Target = Reserve;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct PortObligation(Obligation);

impl PortObligation {
    pub const LEN: usize = Obligation::LEN;
    pub fn calculate_liquidity(
        &self,
        reserve_pubkey: &Pubkey,
        exchange_rate: CollateralExchangeRate,
    ) -> std::result::Result<u64, Error> {
        let borrow = self
            .borrows
            .iter()
            .find_map(|b| {
                if b.borrow_reserve == *reserve_pubkey {
                    Some(b.borrowed_amount_wads)
                } else {
                    None
                }
            })
            .unwrap_or_else(port_variable_rate_lending_instructions::math::Decimal::zero);
        let deposit = self
            .deposits
            .iter()
            .find_map(|b| {
                if b.deposit_reserve == *reserve_pubkey {
                    Some(b.deposited_amount)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        exchange_rate
            .collateral_to_liquidity(deposit)?
            .checked_sub(borrow.try_ceil_u64()?)
            .ok_or(PortAdaptorError::Insolvency.into())
    }
}

impl anchor_lang::AccountDeserialize for PortObligation {
    fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        PortObligation::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        Obligation::unpack(buf)
            .map(PortObligation)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for PortObligation {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> std::result::Result<(), Error> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for PortObligation {
    fn owner() -> Pubkey {
        port_lending_id()
    }
}

impl Deref for PortObligation {
    type Target = Obligation;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct PortStakingPool(StakingPool);

impl PortStakingPool {
    pub const LEN: usize = StakingPool::LEN;
}

impl anchor_lang::AccountDeserialize for PortStakingPool {
    fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        PortStakingPool::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        StakingPool::unpack(buf)
            .map(PortStakingPool)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for PortStakingPool {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> std::result::Result<(), Error> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for PortStakingPool {
    fn owner() -> Pubkey {
        port_staking_id()
    }
}

impl Deref for PortStakingPool {
    type Target = StakingPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct PortLendingMarket(LendingMarket);

impl PortLendingMarket {
    pub const LEN: usize = LendingMarket::LEN;
}

impl anchor_lang::AccountDeserialize for PortLendingMarket {
    fn try_deserialize(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        PortLendingMarket::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> std::result::Result<Self, Error> {
        LendingMarket::unpack(buf)
            .map(PortLendingMarket)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for PortLendingMarket {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> std::result::Result<(), Error> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for PortLendingMarket {
    fn owner() -> Pubkey {
        port_lending_id()
    }
}

impl Deref for PortLendingMarket {
    type Target = LendingMarket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
