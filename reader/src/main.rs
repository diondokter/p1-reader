#![allow(clippy::type_complexity)]

use std::{env, error::Error, io::Read, sync::Arc, time::Duration};

use chrono::{TimeZone, Utc};
use dsmr5::{Tariff, Telegram};
use sqlx::postgres::PgPool;
use tokio::sync::mpsc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let grid_meter_data = Arc::new(std::sync::Mutex::new(
        grid_meter::InstantaneousData::default(),
    ));
    let grid_meter_address = env::var("GRID_METER_ADDRESS")?.parse().unwrap();
    tokio::spawn({
        let grid_meter_data = grid_meter_data.clone();
        async move {
            grid_meter::run_grid_meter_server(grid_meter_address, grid_meter_data)
                .await
                .unwrap();
        }
    });

    let (data_tx, mut data_rx) = mpsc::channel(64);

    println!("Spawning serial port reader");
    std::thread::spawn(|| serial_port_reader(data_tx));

    println!("Connecting to database");
    let pool = PgPool::connect(&env::var("DATABASE_URL")?).await?;
    println!("Running database migrations");
    sqlx::migrate!().run(&pool).await?;
    println!("Ready");

    loop {
        let (electricity_data, slave_data) = data_rx.recv().await.unwrap();

        #[rustfmt::skip]
        {
            let mut grid_meter_data = grid_meter_data.lock().unwrap();
            grid_meter_data.v_l1_n = (electricity_data.voltages[0] * 10.0).round() as i32;
            grid_meter_data.v_l2_n = (electricity_data.voltages[1] * 10.0).round() as i32;
            grid_meter_data.v_l3_n = (electricity_data.voltages[2] * 10.0).round() as i32;

            grid_meter_data.a_l1 = (electricity_data.current[0] * 1000.0).round() as i32;
            grid_meter_data.a_l2 = (electricity_data.current[1] * 1000.0).round() as i32;
            grid_meter_data.a_l3 = (electricity_data.current[2] * 1000.0).round() as i32;

            grid_meter_data.w_l1 = ((electricity_data.active_powers_import[0] - electricity_data.active_powers_export[0]) * 10_000.0).round() as i32;
            grid_meter_data.w_l2 = ((electricity_data.active_powers_import[1] - electricity_data.active_powers_export[1]) * 10_000.0).round() as i32;
            grid_meter_data.w_l3 = ((electricity_data.active_powers_import[2] - electricity_data.active_powers_export[2]) * 10_000.0).round() as i32;
            grid_meter_data.w_sum = grid_meter_data.w_l1 + grid_meter_data.w_l2 + grid_meter_data.w_l3;

            grid_meter_data.kwh_plus_total = ((electricity_data.kwh_import_total_tarif_high + electricity_data.kwh_import_total_tarif_low) * 10.0).round() as i32;
            grid_meter_data.kwh_neg_total = ((electricity_data.kwh_export_total_tarif_high + electricity_data.kwh_export_total_tarif_low) * 10.0).round() as i32;
        };

        sqlx::query!(
            "insert into electricity_data_points values($1, $2, $3, $4, $5, $6, $7, $8)",
            electricity_data.time,
            electricity_data.kwh_import_total_tarif_low,
            electricity_data.kwh_import_total_tarif_high,
            electricity_data.kwh_export_total_tarif_low,
            electricity_data.kwh_export_total_tarif_high,
            &electricity_data.voltages,
            &electricity_data.active_powers_import,
            &electricity_data.active_powers_export,
        )
        .execute(&pool)
        .await?;

        for (i, slave_data) in slave_data.into_iter().enumerate() {
            if let Some(slave_data) = slave_data {
                sqlx::query!(
                    "insert into slave_data_points values($1, $2, $3) ON CONFLICT DO NOTHING",
                    slave_data.time,
                    i as i16,
                    slave_data.value,
                )
                .execute(&pool)
                .await?;
            }
        }
    }
}

fn serial_port_reader(data_tx: mpsc::Sender<(ElectricityData, [Option<SlaveData>; 4])>) {
    let port = serialport::new("/dev/ttyUSB0", 115_200)
        .timeout(Duration::from_millis(2000))
        .open()
        .expect("Failed to open port");

    let reader = dsmr5::Reader::new(port.bytes());

    for readout in reader {
        let readout = match readout {
            Ok(readout) => readout,
            Err(e) => {
                println!("Read error: {e:?}");
                continue;
            }
        };

        let telegram = match readout.to_telegram() {
            Ok(telegram) => telegram,
            Err(e) => {
                println!("Parse error: {e:?}");
                continue;
            }
        };

        let data = match telegram_to_data(telegram) {
            Ok(val) => val,
            Err(e) => {
                println!("Getting data error: {e:?}");
                continue;
            }
        };

        if let Err(e) = data_tx.try_send(data) {
            println!("Could not send data to database handler: {e:?}");
        }
    }
}

fn telegram_to_data(
    telegram: Telegram,
) -> Result<(ElectricityData, [Option<SlaveData>; 4]), Box<dyn Error>> {
    let mut electricity_data = ElectricityData {
        time: chrono::Utc::now(),
        ..Default::default()
    };
    let mut slave_data = [None; 4];

    for obj in telegram.objects() {
        match obj {
            Ok(obj) => match obj {
                dsmr5::OBIS::MeterReadingTo(Tariff::Tariff1, ref val) => {
                    electricity_data.kwh_import_total_tarif_low = f64::from(val) as f32;
                }
                dsmr5::OBIS::MeterReadingTo(Tariff::Tariff2, ref val) => {
                    electricity_data.kwh_import_total_tarif_high = f64::from(val) as f32;
                }
                dsmr5::OBIS::MeterReadingBy(Tariff::Tariff1, ref val) => {
                    electricity_data.kwh_export_total_tarif_low = f64::from(val) as f32;
                }
                dsmr5::OBIS::MeterReadingBy(Tariff::Tariff2, ref val) => {
                    electricity_data.kwh_export_total_tarif_high = f64::from(val) as f32;
                }
                dsmr5::OBIS::InstantaneousVoltage(line, ref val) => {
                    electricity_data.voltages[line as usize] = f64::from(val) as f32;
                }
                dsmr5::OBIS::InstantaneousCurrent(line, ref val) => {
                    electricity_data.current[line as usize] = val.0 as f32;
                }
                dsmr5::OBIS::InstantaneousActivePowerPlus(line, ref val) => {
                    electricity_data.active_powers_import[line as usize] = f64::from(val) as f32;
                }
                dsmr5::OBIS::InstantaneousActivePowerNeg(line, ref val) => {
                    electricity_data.active_powers_export[line as usize] = f64::from(val) as f32;
                }
                dsmr5::OBIS::SlaveMeterReading(s, timestamp, Some(ref val)) => {
                    let offset =
                        chrono::FixedOffset::east_opt(if timestamp.dst { 2 } else { 1 } * 3600)
                            .ok_or_else(|| format!("Timezone error!: {timestamp:?}"))?;

                    slave_data[s as usize] = Some(SlaveData {
                        time: offset
                            .with_ymd_and_hms(
                                timestamp.year as i32 + 2000,
                                timestamp.month as u32,
                                timestamp.day as u32,
                                timestamp.hour as u32,
                                timestamp.minute as u32,
                                timestamp.second as u32,
                            )
                            .latest()
                            .ok_or_else(|| format!("Time error!: {timestamp:?}"))?
                            .to_utc(),
                        value: f64::from(val) as f32,
                    });
                }
                _ => {}
            },
            Err(e) => println!("Obj error: {e:?}"),
        }
    }

    Ok((electricity_data, slave_data))
}

#[derive(Debug, Default)]
struct ElectricityData {
    time: chrono::DateTime<Utc>,

    kwh_import_total_tarif_low: f32,
    kwh_import_total_tarif_high: f32,
    kwh_export_total_tarif_low: f32,
    kwh_export_total_tarif_high: f32,

    voltages: [f32; 3],
    current: [f32; 3],
    active_powers_import: [f32; 3],
    active_powers_export: [f32; 3],
}

#[derive(Debug, Copy, Clone, Default)]
struct SlaveData {
    time: chrono::DateTime<Utc>,
    value: f32,
}
