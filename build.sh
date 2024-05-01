#!/bin/bash

docker compose up db -d
cd reader
sqlx migrate run
cd ../
docker compose build
docker compose down
