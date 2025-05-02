#!/usr/bin/env bash

set -e;

if [ ! -f '.env' ]; then
	echo "missing '.env' file in project root directory" 1>&2;
	exit -1;
fi

export $(grep -v '^#' .env | xargs);

cargo test --tests;

cargo run --example pg_deadpool --features=postgres_tokio;

cargo run --example sqlx --features=postgres_sqlx;
