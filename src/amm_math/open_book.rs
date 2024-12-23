use crate::amm_math::JsonRpcAccountReceiverClient;
use anyhow::{format_err, Result};
use safe_transmute::{
    to_bytes::{transmute_one_to_bytes, transmute_to_bytes},
    transmute_many_pedantic, transmute_one_pedantic,
};
use serum_dex::state::{gen_vault_signer_key, AccountFlag, Market, MarketState, MarketStateV2};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::{
    borrow::Cow,
    convert::{identity, TryFrom},
};

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MarketPubkeys {
    pub market: Box<Pubkey>,
    pub req_q: Box<Pubkey>,
    pub event_q: Box<Pubkey>,
    pub bids: Box<Pubkey>,
    pub asks: Box<Pubkey>,
    pub coin_vault: Box<Pubkey>,
    pub pc_vault: Box<Pubkey>,
    pub vault_signer_key: Box<Pubkey>,
    pub coin_mint: Box<Pubkey>,
    pub pc_mint: Box<Pubkey>,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
}

fn remove_dex_account_padding<'a>(data: &'a [u8]) -> Result<Cow<'a, [u64]>> {
    use serum_dex::state::{ACCOUNT_HEAD_PADDING, ACCOUNT_TAIL_PADDING};
    let head = &data[..ACCOUNT_HEAD_PADDING.len()];
    if data.len() < ACCOUNT_HEAD_PADDING.len() + ACCOUNT_TAIL_PADDING.len() {
        return Err(format_err!("dex account length {} is too small to contain valid padding", data.len()));
    }
    if head != ACCOUNT_HEAD_PADDING {
        return Err(format_err!("dex account head padding mismatch"));
    }
    let tail = &data[data.len() - ACCOUNT_TAIL_PADDING.len()..];
    if tail != ACCOUNT_TAIL_PADDING {
        return Err(format_err!("dex account tail padding mismatch"));
    }
    let inner_data_range = ACCOUNT_HEAD_PADDING.len()..(data.len() - ACCOUNT_TAIL_PADDING.len());
    let inner: &'a [u8] = &data[inner_data_range];
    let words: Cow<'a, [u64]> = match transmute_many_pedantic::<u64>(inner) {
        Ok(word_slice) => Cow::Borrowed(word_slice),
        Err(transmute_error) => {
            let word_vec = transmute_error.copy().map_err(|e| e.without_src())?;
            Cow::Owned(word_vec)
        }
    };
    Ok(words)
}

pub async fn get_keys_for_market<'a>(
    client: Arc<JsonRpcAccountReceiverClient>,
    market_key: &'a Pubkey,
    market: &'a Pubkey,
) -> Result<MarketPubkeys> {
    let account_data = client.get_account_data(&market).await?;
    let words: Cow<[u64]> = remove_dex_account_padding(&account_data)?;
    let market_state: MarketState = {
        let account_flags = Market::account_flags(&account_data)?;
        if account_flags.intersects(AccountFlag::Permissioned) {
            println!("MarketStateV2");
            let state =
                transmute_one_pedantic::<MarketStateV2>(transmute_to_bytes(&words)).map_err(|e| e.without_src())?;
            state.check_flags(true)?;
            state.inner
        } else {
            let state: MarketState =
                transmute_one_pedantic::<MarketState>(transmute_to_bytes(&words)).map_err(|e| e.without_src())?;
            state.check_flags(true)?;
            state
        }
    };
    let vault_signer_key: Pubkey = gen_vault_signer_key(market_state.vault_signer_nonce, market, market_key)?;
    assert_eq!(transmute_to_bytes(&identity(market_state.own_address)), market.as_ref());
    Ok(MarketPubkeys {
        market: Box::new(*market),
        req_q: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.req_q)))?),
        event_q: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.event_q)))?),
        bids: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.bids)))?),
        asks: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.asks)))?),
        coin_vault: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.coin_vault)))?),
        pc_vault: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.pc_vault)))?),
        vault_signer_key: Box::new(vault_signer_key),
        coin_mint: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.coin_mint)))?),
        pc_mint: Box::new(Pubkey::try_from(transmute_one_to_bytes(&identity(market_state.pc_mint)))?),
        coin_lot_size: market_state.coin_lot_size,
        pc_lot_size: market_state.pc_lot_size,
    })
}
