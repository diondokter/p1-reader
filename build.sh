#!/bin/bash

echo "Starting database"
docker compose up db -d --wait
cd reader
echo "Running reader migrations"
sqlx migrate run
cd ../
cd solar-reader
echo "Running solar-reader migrations"
sqlx database create
sqlx migrate run
cd ../
echo "Building docker"
docker compose build
echo "Stopping database"
docker compose down
