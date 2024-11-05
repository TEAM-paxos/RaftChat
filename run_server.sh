#!/usr/bin/env bash

sed -i  's/${address}/addr/g' promtail-config.yml

docker compose down

#docker stop $(docker ps -a | grep raftchat/raftchat | awk '{print $1}')
#docker rm $(docker ps -a | grep raftchat/raftchat | awk '{print $1}')
docker rmi $(docker images | grep raftchat/raftchat |  awk '{print $3}')
docker compose up -d
