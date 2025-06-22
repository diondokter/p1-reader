use std::{env, error::Error, time::Duration};

use sqlx::postgres::PgPool;
use tokio_modbus::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    println!("Connecting to database");
    let pool = PgPool::connect(&env::var("DATABASE_URL")?).await?;
    println!("Running database migrations");
    sqlx::migrate!().run(&pool).await?;

    println!("Getting inverter sock addr");
    let addr = &env::var("INVERTER_SOCKADDR")?.parse()?;
    println!("Ready");

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut ctx = tcp::connect_slave(*addr, Slave(1)).await?;

    #[allow(clippy::eq_op)]
    loop {
        let realtime_data = ctx
            .read_holding_registers(0x0109, 0x0133 - 0x0109)
            .await??;

        let pv1_power = realtime_data[0x09 - 0x09];
        let inv_temp = realtime_data[0x11 - 0x09] as i16 as f32 / 10.0;
        let power = realtime_data[0x13 - 0x09];
        let total_energy = (((realtime_data[0x31 - 0x09] as u32) << 16)
            | realtime_data[0x32 - 0x09] as u32) as f32
            / 100.0;
        let now = chrono::Utc::now();

        interval.tick().await;

        sqlx::query!(
            "insert into solar_data_points values($1, $2, $3, $4, $5)",
            now,
            power as i32,
            pv1_power as i32,
            total_energy,
            inv_temp,
        )
        .execute(&pool)
        .await?;
    }
}
