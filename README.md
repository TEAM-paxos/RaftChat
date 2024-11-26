
<p align="center"><img src="images/raftchat.png" width="300" height="300"></p>


# RaftChat
Chatting system using [raft protocol](https://raft.github.io/)

## Note

https://hackmd.io/@jFaa8ow5QGS_siogO7YAkg/rk66Hkc-Jl

## UI
![alt text](images/image.png)

## Architecture

![alt text](images/image-2.png)
![alt text](images/image-1.png)

## License of dependencies

https://crates.io/crates/cargo-license

```rust 
cargo license
```

## Local server test

```shell
./local_test.sh 0 &
./local_test.sh 1 &
./local_test.sh 2 &
```

client url
- 127.0.0.1:3000
- 127.0.0.1:3001 
- 127.0.0.1:3002


## Git action local test

```shell
touch my.secrets // for git secret key

act --secret-file my.secrets  -P ubuntu-latest=-self-hosted
```