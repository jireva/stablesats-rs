use rust_decimal_macros::dec;
use sqlx::PgPool;
use user_trades::{user_trade::*, user_trade_balances::*, user_trade_unit::*};

lazy_static::lazy_static! {
    static ref POOL: PgPool = {
        let pg_host = std::env::var("USER_TRADES_PG_HOST").unwrap_or("localhost".to_string());
        let pg_con = format!(
            "postgres://stablesats:stablesats@{}:5432/stablesats-user-trades",
            pg_host
        );
        PgPool::connect_lazy(&pg_con).expect("connect to db in user_trade test")
    };
}

#[tokio::test]
async fn user_trade_balances() -> anyhow::Result<()> {
    let units = UserTradeUnits::load(&POOL).await?;
    let balances = UserTradeBalances::new(POOL.clone(), units.clone()).await?;
    let original_balances = balances.fetch_all().await?;

    let trades = UserTrades::new(POOL.clone(), units);

    let sat_amount = dec!(1000);
    let cent_amount = dec!(10);

    let trade = trades
        .persist_new(NewUserTrade::new(
            UserTradeUnit::SynthCent,
            cent_amount,
            UserTradeUnit::Satoshi,
            sat_amount,
        ))
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let new_balances = balances.fetch_all().await?;
    let old_sat_summary = original_balances
        .get(&UserTradeUnit::Satoshi)
        .expect("old sat summary");
    let new_sat_summary = new_balances
        .get(&UserTradeUnit::Satoshi)
        .expect("new sats balance");

    assert_eq!(new_sat_summary.last_trade_idx, Some(trade.idx));
    assert_eq!(
        old_sat_summary.current_balance + sat_amount,
        new_sat_summary.current_balance
    );

    let old_cent_summary = original_balances
        .get(&UserTradeUnit::SynthCent)
        .expect("old cent summary");
    let new_cent_summary = new_balances
        .get(&UserTradeUnit::SynthCent)
        .expect("new cents balance");

    assert_eq!(new_cent_summary.last_trade_idx, Some(trade.idx));
    assert_eq!(
        old_cent_summary.current_balance - cent_amount,
        new_cent_summary.current_balance
    );

    Ok(())
}
