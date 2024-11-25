#!/bin/bash

if [ $# -ne 1  ]; then
    echo "usage: $0 <raft node idx>"
    exit 1
fi

config_file="../config/test.env"
log_file="../config/log4rs.yaml"

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

RANDOM_TOKEN="0.$((RANDOM % 10)).$((RANDOM % 100))"


sed "s/^SELF_DOMAIN_IDX=0$/SELF_DOMAIN_IDX=$1/" $config_file > test$1.env
sed -i "s/^VERSION=.*/VERSION=\"0.0.0\"/" test$1.env
sed -i "s/^REFRESH_TOKEN=\"random_value\"/REFRESH_TOKEN=\"$RANDOM_TOKEN\"/" test$1.env 

./server -c test$1.env -l $log_file

#kill $(ps  | grep server | awk '{print $1}')