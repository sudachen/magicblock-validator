use jsonrpc_pubsub::Subscriber;
use magicblock_geyser_plugin::{
    rpc::GeyserRpcService, types::LogsSubscribeKey,
};
use solana_rpc_client_api::config::RpcTransactionLogsFilter;
use solana_sdk::pubkey::Pubkey;

use super::common::UpdateHandler;
use crate::{
    errors::reject_internal_error,
    notification_builder::LogsNotificationBuilder, types::LogsParams,
};

pub async fn handle_logs_subscribe(
    subid: u64,
    subscriber: Subscriber,
    params: &LogsParams,
    geyser_service: &GeyserRpcService,
) {
    let key = match params.filter() {
        RpcTransactionLogsFilter::All
        | RpcTransactionLogsFilter::AllWithVotes => LogsSubscribeKey::All,
        RpcTransactionLogsFilter::Mentions(pubkeys) => {
            let Some(Ok(pubkey)) =
                pubkeys.first().map(|s| Pubkey::try_from(s.as_str()))
            else {
                reject_internal_error(
                    subscriber,
                    "Invalid Pubkey",
                    Some("failed to base58 decode the provided pubkey"),
                );
                return;
            };
            LogsSubscribeKey::Account(pubkey)
        }
    };
    let mut geyser_rx = geyser_service.logs_subscribe(key, subid).await;
    let builder = LogsNotificationBuilder {};
    let subscriptions_db = geyser_service.subscriptions_db.clone();
    let cleanup = async move {
        subscriptions_db.unsubscribe_from_logs(&key, subid).await;
    };
    let Some(handler) =
        UpdateHandler::new(subid, subscriber, builder, cleanup.into())
    else {
        return;
    };

    while let Some(msg) = geyser_rx.recv().await {
        if !handler.handle(msg) {
            break;
        }
    }
}
