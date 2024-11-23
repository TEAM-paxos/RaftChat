#!/usr/bin/env bash

#shut down case
if [ $1 -ne 1 ]; then 
    docker compose down

    docker rmi $(docker images | grep raftchat/raftchat |  awk '{print $3}')
else  #set up server
    hostname=$(hostname)

    sed -i "s/^SELF_DOMAIN_IDX=0$/SELF_DOMAIN_IDX=$DOMAIN_IDX/" ./config/config.env
    sed -i "s/{address}/$SERVER_ADDRESS/g" promtail-config.yml
    sed -i "s/{hostname}/$hostname/g" promtail-config.yml

    docker compose up -d
fi

