#!/usr/bin/env bash

hostname=$(hostname)
sed -i "s/{address}/$SERVER_ADDRESS/g" promtail-config.yml
sed -i "s/{hostname}/$hostname/g" promtail-config.yml

docker compose down

#docker stop $(docker ps -a | grep raftchat/raftchat | awk '{print $1}')
#docker rm $(docker ps -a | grep raftchat/raftchat | awk '{print $1}')
docker rmi $(docker images | grep raftchat/raftchat |  awk '{print $3}')
docker compose up -d
