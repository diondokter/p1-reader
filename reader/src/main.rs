use std::io::Read;
use std::time::Duration;

use dsmr5::Tariff;

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

        let mut data = Data::default();

        for obj in telegram.objects() {
            match obj {
                Ok(obj) => match obj {
                    dsmr5::OBIS::MeterReadingTo(Tariff::Tariff1, ref val) => {
                        data.kwh_import_total_tarif_low = val.into()
                    }
                    dsmr5::OBIS::MeterReadingTo(Tariff::Tariff2, ref val) => {
                        data.kwh_import_total_tarif_high = val.into()
                    }
                    dsmr5::OBIS::MeterReadingBy(Tariff::Tariff1, ref val) => {
                        data.kwh_export_total_tarif_low = val.into()
                    }
                    dsmr5::OBIS::MeterReadingBy(Tariff::Tariff2, ref val) => {
                        data.kwh_export_total_tarif_high = val.into()
                    }
                    dsmr5::OBIS::InstantaneousVoltage(line, ref val) => {
                        data.voltages[line as usize] = val.into()
                    }
                    dsmr5::OBIS::InstantaneousActivePowerPlus(line, ref val) => {
                        data.active_powers_import[line as usize] = val.into()
                    }
                    dsmr5::OBIS::InstantaneousActivePowerNeg(line, ref val) => {
                        data.active_powers_export[line as usize] = val.into()
                    }
                    dsmr5::OBIS::SlaveMeterReading(_, timestamp, Some(ref val)) => {
                        data.gas_meter = val.into();
                        data.gas_meter_time = timestamp.second as u32
                            + timestamp.minute as u32 * 60
                            + (timestamp.hour as u32 + timestamp.dst as u32) * 3600;
                    }
                    _ => {}
                },
                Err(e) => println!("Obj error: {e:?}"),
            }
        }

        println!("{data:#?}");
    }
}

#[derive(Debug, Default)]
struct Data {
    kwh_import_total_tarif_low: f64,
    kwh_import_total_tarif_high: f64,
    kwh_export_total_tarif_low: f64,
    kwh_export_total_tarif_high: f64,

    voltages: [f64; 3],
    active_powers_import: [f64; 3],
    active_powers_export: [f64; 3],

    gas_meter: f64,
    gas_meter_time: u32,
}
