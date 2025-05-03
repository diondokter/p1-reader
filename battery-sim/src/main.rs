use std::{
    fs::OpenOptions,
    io::{BufReader, Read, Seek, Write},
    path::PathBuf,
    time::Instant,
};

use chrono::{TimeDelta, Utc};
use rayon::slice::ParallelSliceMut;
use serde::Deserialize;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    json_path: PathBuf,
    #[arg(short, long, value_name = "FILE")]
    battery_stats_out: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    println!("Opening file {}...", args.json_path.display());
    let mut json_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(args.json_path)?;

    println!("Loading file...");
    let mut json_data = String::with_capacity(json_file.metadata()?.len() as usize);
    let start = Instant::now();
    BufReader::with_capacity(1024 * 1024 * 64, &json_file).read_to_string(&mut json_data)?;
    println!(
        "Loaded file. Took {}",
        chrono::Duration::from_std(start.elapsed())?
    );

    println!("Checking used time format...");
    if json_data.contains("+00\"") {
        println!("Time format must be changed...");
        let start = Instant::now();
        json_data = json_data.replace("+00\"", "+00:00\"");
        println!(
            "Done. Took {}. Will write back to file",
            chrono::Duration::from_std(start.elapsed())?
        );
        json_file.set_len(0)?;
        json_file.seek(std::io::SeekFrom::Start(0))?;
        json_file.write_all(json_data.as_bytes())?;
        json_file.flush()?;
    }
    println!("Time format is good");

    println!(
        "Sneak peek of json: {:?}",
        json_data.chars().take(100).collect::<String>()
    );

    println!("Parsing file...");
    let start = Instant::now();
    let mut loaded_data = serde_json::from_str::<Vec<ElectricityData>>(&json_data)?;
    println!(
        "Parsed file with {} entries. Took {}",
        loaded_data.len(),
        chrono::Duration::from_std(start.elapsed())?
    );
    // drop(json_data);

    if loaded_data.is_empty() {
        println!("No data available...");
        return Ok(());
    }

    println!("Checking if data is sorted by time...");
    if !loaded_data.is_sorted_by_key(|data| data.time) {
        println!("Sorting is not ok, fixing that now...");
        loaded_data.par_sort_unstable_by_key(|data| data.time);
    }
    println!("Sorting is ok");

    println!("Checking for gaps in data...");
    let mut gaps = 0;
    let mut largest_gap = TimeDelta::zero();
    for data in loaded_data.windows(2) {
        let timediff = data[1].time - data[0].time;
        if timediff > TimeDelta::milliseconds(5000) {
            gaps += 1;
            largest_gap = largest_gap.max(timediff);
        }
    }
    println!("There were {gaps} gaps in total. The largest was {largest_gap}");

    println!("Running baseline sim...");
    let baseline_report = run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 0.0,
            max_charging_rate: 0.0,
            max_discharging_rate: 0.0,
            efficiency: 1.0,
        },
        Strategy::NetZero,
        &loaded_data,
    );
    println!("Done: {baseline_report:?}");

    println!("\nRunning 0.1kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 0.1,
            max_charging_rate: 5.0,
            max_discharging_rate: 5.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 0.2kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 0.2,
            max_charging_rate: 5.0,
            max_discharging_rate: 5.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 0.5kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 0.5,
            max_charging_rate: 5.0,
            max_discharging_rate: 5.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 1kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 1.0,
            max_charging_rate: 5.0,
            max_discharging_rate: 5.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 2kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 2.0,
            max_charging_rate: 5.0,
            max_discharging_rate: 5.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 5kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 5.0,
            max_charging_rate: 5.0,
            max_discharging_rate: 5.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 10kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 10.0,
            max_charging_rate: 5.0,
            max_discharging_rate: 5.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 20kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 20.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 50kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 50.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 100kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 100.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 200kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 200.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 500kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 500.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 1000kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 1000.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 2000kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 2000.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    println!("\nRunning 5000kWh sim...");
    run_battery_sim(
        Battery {
            stored: 0.0,
            capacity: 5000.0,
            max_charging_rate: 10.0,
            max_discharging_rate: 10.0,
            efficiency: 0.95,
        },
        Strategy::NetZero,
        &loaded_data,
    )
    .display_comparison(baseline_report);

    Ok(())
}

#[derive(Debug, Default, Deserialize)]
struct ElectricityData {
    time: chrono::DateTime<Utc>,

    kwh_import_total_tarif_low: f32,
    kwh_import_total_tarif_high: f32,
    kwh_export_total_tarif_low: f32,
    kwh_export_total_tarif_high: f32,

    voltages: [f32; 3],
    active_powers_import: [f32; 3],
    active_powers_export: [f32; 3],
}

impl ElectricityData {
    /// kwh balance, positive for import, negative for export
    fn kwh_balance(&self) -> f32 {
        self.kwh_import_total_tarif_low + self.kwh_import_total_tarif_high
            - self.kwh_export_total_tarif_high
            - self.kwh_export_total_tarif_low
    }
}

struct Battery {
    // kWh
    stored: f32,
    // kWh
    capacity: f32,
    // kW
    max_charging_rate: f32,
    // kW
    max_discharging_rate: f32,
    // 0..1
    efficiency: f32,
}

fn run_battery_sim(
    mut battery: Battery,
    strategy: Strategy,
    mut data_to_go: &[ElectricityData],
) -> Report {
    let mut report = Report {
        total_export: 0.0,
        total_import: 0.0,
    };

    let mut prev = &data_to_go[0];

    while let Some((i, curr)) = data_to_go
        .iter()
        .enumerate()
        .find(|(_, data)| data.time >= prev.time + TimeDelta::minutes(1))
    {
        let charging_goal = strategy.charge_goal(prev, curr);
        let timediff_h = (curr.time - prev.time).as_seconds_f32() / 3600.0;

        let clamped_charging = charging_goal
            .clamp(
                -battery.max_discharging_rate * timediff_h,
                battery.max_charging_rate * timediff_h,
            )
            .clamp(
                -battery.stored / battery.efficiency,
                (battery.capacity - battery.stored) / battery.efficiency,
            );

        battery.stored += clamped_charging * battery.efficiency;

        let real_change_in_period = curr.kwh_balance() - prev.kwh_balance();
        let simmed_change_in_period = real_change_in_period + clamped_charging;

        if simmed_change_in_period > 0.0 {
            report.total_import += simmed_change_in_period;
        } else {
            report.total_export -= simmed_change_in_period;
        }

        prev = curr;
        data_to_go = &data_to_go[i..];
    }

    report
}

#[derive(Clone, Copy)]
struct Report {
    // kWh
    total_import: f32,
    // kWh
    total_export: f32,
}

impl Report {
    const IMPORT_COST: f32 = 0.25;
    const EXPORT_COST: f32 = -0.01;

    fn display_comparison(&self, baseline: Report) {
        println!("-------- Report difference --------");
        println!(
            "Import {:.2} ({:.2})",
            self.total_import,
            self.total_import - baseline.total_import
        );
        println!(
            "Export {:.2} ({:.2})",
            self.total_export,
            self.total_export - baseline.total_export
        );
        println!(
            "Cost €{:.2} (€{:.2})",
            self.total_import * Self::IMPORT_COST + self.total_export * Self::EXPORT_COST,
            self.total_export * Self::EXPORT_COST - baseline.total_export * Self::EXPORT_COST
                + self.total_import * Self::IMPORT_COST
                - baseline.total_import * Self::IMPORT_COST
        );
    }
}

impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Report")
            .field("total_import", &self.total_import)
            .field("total_export", &self.total_export)
            .field(
                "Total cost",
                &(self.total_import * Self::IMPORT_COST + self.total_export * Self::EXPORT_COST),
            )
            .finish()
    }
}

enum Strategy {
    /// Instead of exporting electricity, charge if capacity left
    /// Instead of importing electricity, discharge if storage left
    NetZero,
}

impl Strategy {
    /// kWh, if positive charge, if negative discharge
    fn charge_goal(
        &self,
        previous_situation: &ElectricityData,
        current_situation: &ElectricityData,
    ) -> f32 {
        match self {
            Strategy::NetZero => {
                let change_in_period =
                    current_situation.kwh_balance() - previous_situation.kwh_balance();
                -change_in_period
            }
        }
    }
}
