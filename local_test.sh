#!/bin/bash

config_file="../config/test.env"
log_file="../config/log4rs.yaml"

pids=()

cargo build
rm -r test$1
mkdir test$1 && cd test$1 && mkdir logs
cp ../target/debug/server .
cp -r ../client .

if [ ! -f "$config_file" ]; then
  echo "Error: Template file '$config_file' not found!"
  exit 1
fi

if [ ! -f "$log_file" ]; then
  echo "Error: Template file '$log_file' not found!"
  exit 1
fi

sed "s/^SELF_DOMAIN_IDX=0$/SELF_DOMAIN_IDX=$1/" $config_file > test$1.env
  ./server -c test$1.env -l $log_file &
  pids+=($!);

echo "Started processes with the following PIDs:"
for pid in "${pids[@]}"; do
  echo "$pid"
done

#kill $(ps  | grep server | awk '{print $1}')