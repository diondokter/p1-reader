-- Add up migration script here

CREATE TABLE solar_data_points (
	time TIMESTAMPTZ PRIMARY KEY,

	-- delivered to net
	active_power_output INTEGER NOT NULL,
	-- delivered from panels
	active_power_input INTEGER NOT NULL,
	-- total kWh counter 
	total_energy REAL NOT NULL,
	-- In C
	inverter_temperature SMALLINT NOT NULL
);
