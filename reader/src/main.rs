use std::time::Duration;
use std::{error::Error, io::Read};

use chrono::{TimeZone, Utc};
use dsmr5::{Tariff, Telegram};

fn main() {
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

        println!("\n---------------------------\n");

        let (electricity_data, slave_data) = match telegram_to_data(telegram) {
            Ok(val) => val,
            Err(e) => {
                println!("Getting data error: {e:?}");
                continue;
            }
        };

        println!("{electricity_data:#?}");
        println!("{slave_data:#?}");
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
                    electricity_data.kwh_import_total_tarif_low = val.into()
                }
                dsmr5::OBIS::MeterReadingTo(Tariff::Tariff2, ref val) => {
                    electricity_data.kwh_import_total_tarif_high = val.into()
                }
                dsmr5::OBIS::MeterReadingBy(Tariff::Tariff1, ref val) => {
                    electricity_data.kwh_export_total_tarif_low = val.into()
                }
                dsmr5::OBIS::MeterReadingBy(Tariff::Tariff2, ref val) => {
                    electricity_data.kwh_export_total_tarif_high = val.into()
                }
                dsmr5::OBIS::InstantaneousVoltage(line, ref val) => {
                    electricity_data.voltages[line as usize] = val.into()
                }
                dsmr5::OBIS::InstantaneousActivePowerPlus(line, ref val) => {
                    electricity_data.active_powers_import[line as usize] = val.into()
                }
                dsmr5::OBIS::InstantaneousActivePowerNeg(line, ref val) => {
                    electricity_data.active_powers_export[line as usize] = val.into()
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
                        value: val.into(),
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

    kwh_import_total_tarif_low: f64,
    kwh_import_total_tarif_high: f64,
    kwh_export_total_tarif_low: f64,
    kwh_export_total_tarif_high: f64,

    voltages: [f64; 3],
    active_powers_import: [f64; 3],
    active_powers_export: [f64; 3],
}

#[derive(Debug, Copy, Clone, Default)]
struct SlaveData {
    time: chrono::DateTime<Utc>,
    value: f64,
}
