use std::{env, net::SocketAddr, time::Duration};

use backoff::backoff::Backoff;
use sqlx::{Pool, Postgres, postgres::PgPool};
use tokio::time::timeout;
use tokio_modbus::client::Reader;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    println!("Connecting to database");
    let pool = PgPool::connect(&env::var("DATABASE_URL")?).await?;
    println!("Running database migrations");
    sqlx::migrate!().run(&pool).await?;

    println!("Getting inverter sock addr");
    let addr = &env::var("INVERTER_SOCKADDR")?.parse()?;
    println!("Ready");

    let mut backoff = backoff::ExponentialBackoffBuilder::new()
        .with_max_interval(Duration::from_secs(300))
        .with_initial_interval(Duration::from_secs(1))
        .with_multiplier(1.5)
        .with_max_elapsed_time(None)
        .build();

    loop {
        let connect_time = tokio::time::Instant::now();

        let result = connect_and_run(*addr, &pool).await;
        println!("Connection ended with: {}", result.as_ref().unwrap_err());

        match result {
            e @ Err(Error::Sqlx(_)) => e?,
            _ => {
                if connect_time.elapsed() > Duration::from_secs(120) {
                    backoff.reset();
                }

                tokio::time::sleep(backoff.next_backoff().unwrap()).await;
            }
        }
    }
}

async fn connect_and_run(addr: SocketAddr, pool: &Pool<Postgres>) -> Result<(), Error> {
    println!("Trying to connect to: {addr}");
    let mut ctx = timeout(
        Duration::from_secs(60),
        tokio_modbus::client::tcp::connect_slave(addr, tokio_modbus::Slave(1)),
    )
    .await??;

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

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
        .execute(pool)
        .await?;
    }
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Modbus error: {0}")]
    Modbus(#[from] tokio_modbus::Error),
    #[error("ExceptionCode error: {0}")]
    ExceptionCode(#[from] tokio_modbus::ExceptionCode),
    #[error("Timeout: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),
}
