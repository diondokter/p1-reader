#!/bin/bash

echo "Starting database"
docker compose up db -d --wait
cd reader
echo "Running migrations"
sqlx migrate run
cd ../
echo "Building docker"
docker compose build
echo "Stopping database"
docker compose down
