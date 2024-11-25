#!/usr/bin/env bash

#shut down case
if [ $1 -ne 1 ]; then 
    docker compose down

    docker rmi -f $(docker images | grep raftchat/raftchat |  awk '{print $3}')
else  #set up server
    hostname=$(hostname)

    RANDOM_TOKEN="$((RANDOM % 10)).$((RANDOM % 10)).$((RANDOM % 100))"

    sed -i "s/^SELF_DOMAIN_IDX=0$/SELF_DOMAIN_IDX=$DOMAIN_IDX/" ./config/config.env
    sed -i "s/{address}/$SERVER_ADDRESS/g" promtail-config.yml
    sed -i "s/{hostname}/$hostname/g" promtail-config.yml
    sed -i "s/^REFRESH_TOKEN=\"random_value\"/REFRESH_TOKEN=\"$RANDOM_TOKEN\"/" ./config/config.env 

    docker compose up -d
fi

