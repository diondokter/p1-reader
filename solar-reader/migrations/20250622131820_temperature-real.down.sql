-- Add down migration script here

ALTER TABLE solar_data_points
ALTER COLUMN inverter_temperature TYPE SMALLINT
USING inverter_temperature::SMALLINT;
