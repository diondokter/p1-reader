#!/bin/bash

docker compose up db -d --wait
cd reader
sqlx migrate run
cd ../
docker compose build
docker compose down
