#!/bin/bash
while ! cargo run --release -- --home-dir ~/.near/testnet run
do
  sleep 1
  echo "Restarting program..."
done
