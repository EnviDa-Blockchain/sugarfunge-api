use crate::state::*;
use crate::sugarfunge;
use crate::util::*;
use actix_web::{error, web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;
use subxt::PairSigner;
use sugarfunge::runtime_types::sugarfunge_primitives::CurrencyId;

#[derive(Deserialize)]
pub struct IssueTokenInput {
    input: IssueTokenArg,
}

#[derive(Deserialize)]
pub struct IssueTokenArg {
    seed: String,
    account: String,
    token_id: u64,
    amount: i128,
}

impl Into<u64> for CurrencyId {
    fn into(self) -> u64 {
        match self {
            CurrencyId::Token(token) => token as u64,
            CurrencyId::Id(id) => id,
        }
    }
}

#[derive(Serialize)]
pub struct IssueTokenOutput {
    token_id: u64,
    account_id: String,
    amount: i128,
}

/// Issue amount of token id
pub async fn issue(
    data: web::Data<AppState>,
    req: web::Json<IssueTokenInput>,
) -> error::Result<HttpResponse> {
    let pair = get_pair_from_seed(&req.input.seed)?;
    let signer = PairSigner::new(pair);

    let account = sp_core::sr25519::Public::from_str(&req.input.account);
    if let Ok(account) = account {
        let account = sp_core::crypto::AccountId32::from(account);
        let account = subxt::sp_runtime::MultiAddress::Id(account);

        let currency_id =
            sugarfunge::runtime_types::sugarfunge_primitives::CurrencyId::Id(req.input.token_id);

        let call = sugarfunge::runtime_types::sugarfunge_runtime::Call::OrmlCurrencies(
            sugarfunge::runtime_types::orml_currencies::module::Call::update_balance {
                who: account,
                currency_id,
                amount: req.input.amount,
            },
        );

        let api = data.api.lock().unwrap();
        let result = api
            .tx()
            .sudo()
            .sudo(call)
            .sign_and_submit_then_watch(&signer)
            .await
            .map_err(map_subxt_err)?;

        let result = result
            .find_event::<sugarfunge::orml_currencies::events::BalanceUpdated>()
            .map_err(map_scale_err)?;

        match result {
            Some(event) => Ok(HttpResponse::Ok().json(IssueTokenOutput {
                token_id: event.0.into(),
                account_id: event.1.to_string(),
                amount: event.2,
            })),
            None => Ok(HttpResponse::BadRequest().json(RequestError {
                message: json!(
                    "Failed to find sugarfunge::orml_currencies::events::BalanceUpdated"
                ),
            })),
        }
    } else {
        Ok(HttpResponse::BadRequest().json(RequestError {
            message: json!("Invalid account"),
        }))
    }
}

#[derive(Serialize, Deserialize)]
pub struct TokenIssuanceInput {
    input: TokenIssuanceArg,
}

#[derive(Serialize, Deserialize)]
pub struct TokenIssuanceArg {
    token_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TokenIssuanceOutput {
    amount: u128,
}

/// Get total issuance for given token id
pub async fn issuance(
    data: web::Data<AppState>,
    req: web::Json<TokenIssuanceInput>,
) -> error::Result<HttpResponse> {
    let api = data.api.lock().unwrap();

    let currency_id = CurrencyId::Id(req.input.token_id);
    let result = api
        .storage()
        .orml_tokens()
        .total_issuance(currency_id, None)
        .await;

    let amount = result.map_err(map_subxt_err)?;
    Ok(HttpResponse::Ok().json(TokenIssuanceOutput { amount }))
}

#[derive(Serialize, Deserialize)]
pub struct MintTokenInput {
    input: MintTokenArg,
}

#[derive(Serialize, Deserialize)]
pub struct MintTokenArg {
    seed: String,
    token_id: u64,
    amount: u128,
}

#[derive(Serialize, Deserialize)]
pub struct MintTokenOutput {
    token_id: u64,
    amount: u128,
    account_id: String,
}

/// Mint amount of token id
pub async fn mint(
    data: web::Data<AppState>,
    req: web::Json<MintTokenInput>,
) -> error::Result<HttpResponse> {
    let pair = get_pair_from_seed(&req.input.seed)?;
    let signer = PairSigner::new(pair);

    let currency_id = CurrencyId::Id(req.input.token_id);

    let api = data.api.lock().unwrap();
    let result = api
        .tx()
        .currency()
        .mint(currency_id, req.input.amount)
        .sign_and_submit_then_watch(&signer)
        .await
        .map_err(map_subxt_err)?;

    let result = result
        .find_event::<sugarfunge::currency::events::TokenMint>()
        .map_err(map_scale_err)?;

    match result {
        Some(event) => Ok(HttpResponse::Ok().json(MintTokenOutput {
            token_id: event.0.into(),
            amount: event.1,
            account_id: event.2.to_string(),
        })),
        None => Ok(HttpResponse::BadRequest().json(RequestError {
            message: json!("Failed to find sugarfunge::currency::events::TokenMint"),
        })),
    }
}

#[derive(Serialize, Deserialize)]
pub struct TokenBalanceInput {
    input: TokenBalanceArg,
}

#[derive(Serialize, Deserialize)]
pub struct TokenBalanceArg {
    account: String,
    token_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TokenBalanceOutput {
    amount: u128,
}

/// Get balance for given token id
pub async fn balance(
    data: web::Data<AppState>,
    req: web::Json<TokenBalanceInput>,
) -> error::Result<HttpResponse> {
    let account = sp_core::sr25519::Public::from_str(&req.input.account);
    if let Ok(account) = account {
        let account = sp_core::crypto::AccountId32::from(account);
        let api = data.api.lock().unwrap();
        let result = api
            .storage()
            .token()
            .balances(account, (0, req.input.token_id), None)
            .await;
        let amount = result.map_err(map_subxt_err)?;
        Ok(HttpResponse::Ok().json(TokenBalanceOutput { amount }))
    } else {
        Ok(HttpResponse::BadRequest().json(RequestError {
            message: json!("Invalid account"),
        }))
    }
}