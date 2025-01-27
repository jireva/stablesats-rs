use futures::stream::StreamExt;
use rust_decimal_macros::dec;
use std::fs;

use price_server::{app::*, *};
use shared::{payload::*, pubsub::*, time::*};

#[derive(serde::Deserialize)]
struct Fixture {
    payload: OrderBookPayload,
}

fn load_fixture(dataname: &str) -> anyhow::Result<Fixture> {
    let contents = fs::read_to_string(format!(
        "./tests/fixtures/order-book-payload-{}.json",
        dataname
    ))
    .expect("Couldn't load fixtures");
    Ok(serde_json::from_str(&contents)?)
}

#[tokio::test]
async fn price_app_with_order_book_cache() -> anyhow::Result<()> {
    let redis_host = std::env::var("REDIS_HOST").unwrap_or("localhost".to_string());
    let config = PubSubConfig {
        host: Some(redis_host),
        ..PubSubConfig::default()
    };
    let publisher = Publisher::new(config.clone()).await?;
    let subscriber = Subscriber::new(config.clone()).await?;
    let mut stream = subscriber
        .subscribe::<OkexBtcUsdSwapOrderBookPayload>()
        .await?;

    let (_, recv) = futures::channel::mpsc::unbounded();
    let app = PriceApp::run(
        recv,
        FeeCalculatorConfig {
            base_fee_rate: dec!(0.001),
            immediate_fee_rate: dec!(0.01),
            delayed_fee_rate: dec!(0.1),
        },
        config,
    )
    .await?;

    let err = app
        .get_cents_from_sats_for_immediate_buy(Sats::from_major(100_000_000))
        .await;
    if let Err(PriceAppError::OrderBookCacheError(OrderBookCacheError::NoSnapshotAvailable)) = err {
        assert!(true)
    } else {
        assert!(false)
    }

    let mut payload = load_fixture("real")?.payload;

    publisher
        .publish(OkexBtcUsdSwapOrderBookPayload(payload.clone()))
        .await?;
    stream.next().await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let err = app
        .get_cents_from_sats_for_immediate_buy(Sats::from_major(100))
        .await;
    if let Err(PriceAppError::OrderBookCacheError(OrderBookCacheError::OutdatedSnapshot(_))) = err {
        assert!(true)
    } else {
        assert!(false)
    }

    payload.timestamp = TimeStamp::now();
    publisher
        .publish(OkexBtcUsdSwapOrderBookPayload(payload))
        .await?;
    stream.next().await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let cents = app
        .get_cents_from_sats_for_immediate_buy(Sats::from_major(100))
        .await?;
    assert_eq!(cents, UsdCents::from_major(15));
    let cents = app
        .get_cents_from_sats_for_immediate_buy(Sats::from_major(1))
        .await?;
    assert_eq!(cents, UsdCents::from_major(0));

    let cents = app
        .get_cents_from_sats_for_immediate_sell(Sats::from_major(100))
        .await?;
    assert_eq!(cents, UsdCents::from_major(20));
    let cents = app
        .get_cents_from_sats_for_immediate_sell(Sats::from_major(1))
        .await?;
    assert_eq!(cents, UsdCents::from_major(1));

    let cents = app
        .get_cents_from_sats_for_future_buy(Sats::from_major(100))
        .await?;
    assert_eq!(cents, UsdCents::from_major(13));
    let cents = app
        .get_cents_from_sats_for_future_buy(Sats::from_major(1))
        .await?;
    assert_eq!(cents, UsdCents::from_major(0));

    let future_buy = app
        .get_cents_from_sats_for_future_sell(Sats::from_major(100))
        .await?;
    assert_eq!(future_buy, UsdCents::from_major(21));
    let future_buy = app
        .get_cents_from_sats_for_future_sell(Sats::from_major(1))
        .await?;
    assert_eq!(future_buy, UsdCents::from_major(1));

    let sats = app
        .get_sats_from_cents_for_immediate_buy(UsdCents::from_major(10))
        .await?;
    assert_eq!(sats, Sats::from_major(51));

    let sats = app
        .get_sats_from_cents_for_immediate_sell(UsdCents::from_major(10))
        .await?;
    assert_eq!(sats, Sats::from_major(98));
    let sats = app
        .get_sats_from_cents_for_immediate_sell(UsdCents::from_major(1))
        .await?;
    assert_eq!(sats, Sats::from_major(9));

    let sats = app
        .get_sats_from_cents_for_future_buy(UsdCents::from_major(10))
        .await?;
    assert_eq!(sats, Sats::from_major(56));
    let sats = app
        .get_sats_from_cents_for_future_buy(UsdCents::from_major(1))
        .await?;
    assert_eq!(sats, Sats::from_major(6));

    let sats = app
        .get_sats_from_cents_for_future_sell(UsdCents::from_major(10))
        .await?;
    assert_eq!(sats, Sats::from_major(89));
    let sats = app
        .get_sats_from_cents_for_future_sell(UsdCents::from_major(1))
        .await?;
    assert_eq!(sats, Sats::from_major(8));

    let ratio = app.get_cents_per_sat_exchange_mid_rate().await?;
    assert_eq!(ratio, 0.15);

    Ok(())
}
