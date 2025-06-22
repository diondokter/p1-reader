-- Add up migration script here

CREATE TABLE IF NOT EXISTS electricity_data_points (
	time TIMESTAMPTZ PRIMARY KEY,

	kwh_import_total_tarif_low REAL NOT NULL,
	kwh_import_total_tarif_high REAL NOT NULL,
	kwh_export_total_tarif_low REAL NOT NULL,
	kwh_export_total_tarif_high REAL NOT NULL,

	voltages REAL[3] NOT NULL,
	active_powers_import REAL[3] NOT NULL,
	active_powers_export REAL[3] NOT NULL
);

CREATE TABLE IF NOT EXISTS slave_data_points (
	time TIMESTAMPTZ,
    id smallint,
    value REAL NOT NULL,

    PRIMARY KEY(time, id)
);