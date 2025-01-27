mod book;

use futures::{SinkExt, Stream, StreamExt};
use std::pin::Pin;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::{config::*, error::*};
pub use book::*;

pub async fn subscribe_btc_usd_swap_order_book(
    PriceFeedConfig { url }: PriceFeedConfig,
) -> Result<Pin<Box<dyn Stream<Item = OkexOrderBook> + Send>>, PriceFeedError> {
    let (ws_stream, _ws_sink) = connect_async(url).await?;
    let (mut sender, receiver) = ws_stream.split();

    let subscribe_args = serde_json::json!({
        "op": "subscribe",
        "args": [
           {
                "channel": "books",
                "instId": "BTC-USD-SWAP"
            }
        ]
    })
    .to_string();
    let item = Message::Text(subscribe_args);
    sender.send(item).await?;

    Ok(Box::pin(receiver.filter_map(|message| async {
        if let Ok(msg) = message {
            if let Ok(msg_str) = msg.into_text() {
                if let Ok(book) = serde_json::from_str::<OkexOrderBook>(&msg_str) {
                    return Some(book);
                }
            }
        }
        None
    })))
}
