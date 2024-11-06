FROM rust:1.81-slim-bullseye AS chef

WORKDIR /usr/src/raftchat

RUN set -eux; \ 
    cargo install cargo-chef; \
    rm -rf $CARGO_HOME/registry

## build cargo chef recipe
FROM chef AS planner 
COPY . . 
RUN cargo chef prepare --recipe-path recipe.json

## build
FROM chef AS builder

COPY --from=planner /usr/src/raftchat/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json 

COPY . . 
RUN apt update -y && apt upgrade -y
RUN apt install -y protobuf-compiler libprotobuf-dev  
RUN  cargo build -p server --release 

## 

FROM debian:bullseye-slim

WORKDIR /usr/local/bin 

COPY --from=builder /usr/src/raftchat/client ./client
COPY --from=builder /usr/src/raftchat/config ./config
COPY --from=builder /usr/src/raftchat/target/release/server ./raftchat/

CMD [ "/bin/bash", "-c", "cd ./raftchat && ./server"]