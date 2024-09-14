#!/bin/sh

source prepare-sqlx.env
refinery migrate -e DATABASE_URL -p ./migrations
cargo sqlx prepare
