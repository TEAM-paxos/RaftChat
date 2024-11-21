#!/bin/bash

config_file="../config/test.env"
log_file="../config/log4rs.yaml"

count=${1:-3} 

pids=()

cargo build
rm -r test
mkdir test && cd test && mkdir logs
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

echo "Running $count servers..."

# 반복문 실행
for ((i = 0; i < $count; i++)); do
  sed "s/^SELF_DOMAIN_IDX=0$/SELF_DOMAIN_IDX=$i/" $config_file > test$i.env
  ./server -c test$i.env -l $log_file &
  pids+=($!);
done

echo "Started processes with the following PIDs:"
for pid in "${pids[@]}"; do
  echo "$pid"
done