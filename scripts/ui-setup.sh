#!/usr/bin/env bash

# clear out federation startup configs folder
rm -rf $PWD/fedimint-server/setup

# start bitcoind in the background
bitcoind -regtest -daemon -fallbackfee=0.0004 -txindex -server -rpcuser=bitcoin -rpcpassword=bitcoin

# start 5 guardians, kill processes on their ports if they are already running

for ((ID = 0; ID < 4; ID++)); do
  npx kill-port $((10000 + $ID))
  cargo run --bin fedimintd $PWD/../setup/mint-$ID.json $PWD/../setup/mint-$ID.db $((10000 + $ID)) &
done

function kill_fedimint_processes {
  pkill "fedimintd" || true
}

trap kill_fedimint_processes EXIT
