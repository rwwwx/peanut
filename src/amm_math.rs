mod open_book;

use std::sync::Arc;

use crate::amm_math::open_book::{get_keys_for_market, MarketPubkeys};
use crate::rpc::JsonRpcAccountReceiver;
use arrayref::array_ref;
use raydium_amm::math::Calculator;
use raydium_amm::processor::Processor;
use raydium_amm::state::AmmInfo;
use raydium_amm::{processor, state::AmmStatus};
use solana_program::{
    account_info::{AccountInfo, IntoAccountInfo},
    program_pack::Pack,
};
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Account;

pub(crate) type JsonRpcAccountReceiverClient = dyn JsonRpcAccountReceiver + Send + Sync;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct AmmKeys {
    pub amm_pool: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_target: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub amm_open_order: Pubkey,
    pub market_program: Pubkey,
    pub market: Pubkey,
    pub nonce: u8,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CalculateResult {
    pub pool_pc_vault_amount: u64,
    pub pool_pc_decimals: u64,
    pub pool_coin_vault_amount: u64,
    pub pool_coin_decimals: u64,
    pub pool_lp_amount: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PoolState {
    pub pool: CalculateResult,
    pub pool_amm_keys: AmmKeys,
    pub pool_market_keys: MarketPubkeys,
}

pub async fn calc_pool_valut_amounts(
    client: Arc<JsonRpcAccountReceiverClient>,
    amm_program_key: &Pubkey,
    amm_pool_key: &Pubkey,
    amm_keys: &AmmKeys,
    market_keys: &MarketPubkeys,
    amm: &AmmInfo,
) -> anyhow::Result<CalculateResult> {
    let load_pubkeys = vec![
        *amm_pool_key,
        amm_keys.amm_target,
        amm_keys.amm_pc_vault,
        amm_keys.amm_coin_vault,
        amm_keys.amm_open_order,
        amm_keys.market,
        *market_keys.event_q,
    ];

    let accounts = client.get_multiple_accounts(&load_pubkeys).await?;
    let accounts = array_ref![accounts, 0, 7];
    let [_, _, amm_pc_vault_account, amm_coin_vault_account, amm_open_orders_account, market_account, market_event_q_account] =
        accounts;

    let amm_pc_vault = Account::unpack(&amm_pc_vault_account.data)?;
    let amm_coin_vault = Account::unpack(&amm_coin_vault_account.data)?;

    let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
        if AmmStatus::from_u64(amm.status).orderbook_permission() {
            let amm_open_orders_account = &mut amm_open_orders_account.clone();
            let market_account = &mut market_account.clone();
            let market_event_q_account = &mut market_event_q_account.clone();

            let amm_open_orders_info = (&amm.open_orders, amm_open_orders_account).into_account_info();
            let market_account_info = (&amm.market, market_account).into_account_info();
            let market_event_queue_info = (&(*market_keys.event_q), market_event_q_account).into_account_info();

            let amm_authority = Pubkey::find_program_address(&[processor::AUTHORITY_AMM], &amm_program_key).0;
            let lamports = &mut 0;
            let data = &mut [0u8];
            let owner = Pubkey::default();
            let amm_authority_info = AccountInfo::new(&amm_authority, false, false, lamports, data, &owner, false, 0);
            let (market_state, open_orders) = Processor::load_serum_market_order(
                &market_account_info,
                &amm_open_orders_info,
                &amm_authority_info,
                &amm,
                false,
            )?;
            let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) = Calculator::calc_total_without_take_pnl(
                amm_pc_vault.amount,
                amm_coin_vault.amount,
                &open_orders,
                &amm,
                &market_state,
                &market_event_queue_info,
                &amm_open_orders_info,
            )?;
            (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount)
        } else {
            let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
                Calculator::calc_total_without_take_pnl_no_orderbook(amm_pc_vault.amount, amm_coin_vault.amount, &amm)?;
            (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount)
        };

    Ok(CalculateResult {
        pool_pc_vault_amount: amm_pool_pc_vault_amount,
        pool_pc_decimals: amm.pc_decimals,
        pool_coin_vault_amount: amm_pool_coin_vault_amount,
        pool_coin_decimals: amm.coin_decimals,
        pool_lp_amount: amm.lp_amount,
        swap_fee_numerator: amm.fees.swap_fee_numerator,
        swap_fee_denominator: amm.fees.swap_fee_denominator,
    })
}

pub fn load_amm_keys(amm_program_key: &Pubkey, amm_pool_key: &Pubkey, amm_info: &AmmInfo) -> anyhow::Result<AmmKeys> {
    Ok(AmmKeys {
        amm_pool: *amm_pool_key,
        amm_target: amm_info.target_orders,
        amm_coin_vault: amm_info.coin_vault,
        amm_pc_vault: amm_info.pc_vault,
        amm_lp_mint: amm_info.lp_mint,
        amm_open_order: amm_info.open_orders,
        amm_coin_mint: amm_info.coin_vault_mint,
        amm_pc_mint: amm_info.pc_vault_mint,
        amm_authority: Processor::authority_id(amm_program_key, processor::AUTHORITY_AMM, amm_info.nonce as u8)?,
        market: amm_info.market,
        market_program: amm_info.market_program,
        nonce: amm_info.nonce as u8,
    })
}

pub async fn load_pool_state(
    client: Arc<JsonRpcAccountReceiverClient>,
    amm_info: Vec<u8>,
    amm_program_key: &Pubkey,
    amm_pool_key: &Pubkey,
) -> anyhow::Result<PoolState> {
    let amm_info = {
        let account_data = amm_info.as_slice();
        unsafe { &*(&account_data[0] as *const u8 as *const AmmInfo) }.clone()
    };
    let amm_keys = load_amm_keys(&amm_program_key, &amm_pool_key, &amm_info)?;
    let market_keys = get_keys_for_market(client.clone(), &amm_keys.market_program, &amm_keys.market).await?;
    let calculate_result =
        calc_pool_valut_amounts(client.clone(), &amm_program_key, &amm_pool_key, &amm_keys, &market_keys, &amm_info)
            .await?;

    Ok(PoolState { pool: calculate_result, pool_amm_keys: amm_keys, pool_market_keys: market_keys })
}

pub fn calc_coin_in_pc(pool: &CalculateResult) -> f64 {
    (pool.pool_pc_vault_amount as f64)
        / 10_f64.powf(pool.pool_pc_decimals as f64)
        / (pool.pool_coin_vault_amount as f64)
        * 10_f64.powf(pool.pool_coin_decimals as f64)
}
