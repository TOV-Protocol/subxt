// Copyright 2019-2022 Parity Technologies (UK) Ltd.
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

use futures::join;
use sp_keyring::AccountKeyring;
use subxt::{
    ClientBuilder,
    DefaultConfig,
    PolkadotExtrinsicParams,
};

#[subxt::subxt(runtime_metadata_path = "../artifacts/polkadot_metadata.scale")]
pub mod polkadot {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = ClientBuilder::new()
        .build()
        .await?
        .to_runtime_api::<polkadot::RuntimeApi<DefaultConfig, PolkadotExtrinsicParams<DefaultConfig>>>();

    let addr = AccountKeyring::Bob.to_account_id();

    // For storage requests, we can join futures together to
    // await multiple futures concurrently:
    let a_fut = api.storage().staking().bonded(&addr, None);
    let b_fut = api.storage().staking().ledger(&addr, None);
    let (a, b) = join!(a_fut, b_fut);

    println!("{a:?}, {b:?}");

    Ok(())
}
