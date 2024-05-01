#!/bin/bash

docker compose up db -d
sqlx migrate run
(export DATABASE_URL="postgres://postgres:psqlpassword@db/p1-data"; docker compose build)
docker compose down
