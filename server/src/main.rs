use axum::Extension;
use axum::{routing::get, Router};
use clap::Parser;
use futures_util::stream::SplitSink;
use log::{info, set_logger};
use raft::persistent_state::PersistentState;
use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tower_http::services::ServeDir;

mod axum_handler;
mod data_model;
mod events;

#[derive(Clone, serde::Serialize, Debug)]
struct Config {
    domains: Vec<String>, // [TODO] change to Vec<&str>
    web_ports: Vec<u16>,
    socket_ports: Vec<u16>,
    rpc_ports: Vec<u16>,
    version: String,
    self_domain_idx: usize,
    raft_mock_flag: bool
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// path of config.env
    #[arg(short, long, value_name = "config path")]
    config: Option<PathBuf>,

    /// path of log4rs.yml
    #[arg(short, long, value_name = "log yaml path")]
    log: Option<PathBuf>,

    /// debug mode 
    #[arg(short, long, value_name = "debug flag")]
    debug: bool,

    /// raft mock flag
    #[arg(short, long, value_name = "debug flag")]
    raft_mock_flag: bool,
}

async fn setup() -> Config {
    let cli = Cli::parse();

    let config_path = cli
        .config
        .unwrap_or_else(|| PathBuf::from("./config/config.env"));
    let log_config_path = cli
        .log
        .unwrap_or_else(|| PathBuf::from("./config/log4rs.yaml"));

    // config setting
    dotenv::from_path(config_path).unwrap();
    // log setting
    log4rs::init_file(log_config_path, Default::default()).unwrap();

    let self_domain_idx: usize = env::var("SELF_DOMAIN_IDX")
        .ok()
        .and_then(|val| val.parse::<usize>().ok())
        .unwrap_or(0);

    let domains: Vec<String> = env::var("DOMAINS")
        .unwrap()
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let web_ports: Vec<u16> = env::var("WEB_PORT")
        .unwrap()
        .split(',')
        .map(|val| val.parse::<u16>().unwrap_or(3001))
        .collect();

    let socket_ports: Vec<u16> = env::var("SOCKET_PORT")
        .unwrap()
        .split(',')
        .map(|val| val.parse::<u16>().unwrap_or(9001))
        .collect();

    let rpc_ports:  Vec<u16> = env::var("RPC_PORT")
        .unwrap()
        .split(',')
        .map(|val| val.parse::<u16>().unwrap_or(9001))
        .collect();

    let version: String = env::var("VERSION").unwrap();

    return Config {
        raft_mock_flag : cli.raft_mock_flag,
        domains,
        web_ports,
        socket_ports,
        rpc_ports,
        version,
        self_domain_idx
    };
}

async fn run_axum(config: &Config) {
    // axum server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.web_ports[config.self_domain_idx]));
    let app = Router::new()
        .route("/", get(axum_handler::handler))
        .nest_service("/static", ServeDir::new("./client/static"))
        .route("/get_info", get(axum_handler::get_info))
        .layer(Extension(config.clone()));

    let listener = TcpListener::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        info!("AXUM listening on {}", addr);
        axum::serve(listener, app).await.unwrap();
    });
}

// - make channel from Database like raft or sync db
// - start server read write tasks
async fn run_tasks(
    hash: Arc<tokio::sync::Mutex<HashMap<String, u64>>>,
    config: &Config,
) -> (
    Sender<(String, data_model::msg::ClientMsg)>,
    Sender<(String, SplitSink<WebSocketStream<TcpStream>, Message>)>,
) {
    let raft_config = raft::RaftConfig {
        // rpc address
        serve_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), config.rpc_ports[config.self_domain_idx]),
        // unique ID for raft node
        self_id: format!("http://{}:{}", config.domains[config.self_domain_idx], config.rpc_ports[config.self_domain_idx]).leak() as &'static str, 
        peers: config.domains.clone()
        .into_iter()
        .enumerate()
        .filter_map(|(idx, s)| {
            if idx == config.self_domain_idx {
                None
            } else {
                Some(format!("http://{}:{}", s, config.rpc_ports[idx]).leak() as &'static str)
            }
        }).collect(),
        election_duration: tokio::time::Duration::from_millis(1000),
        heartbeat_duration: tokio::time::Duration::from_millis(250),
        persistent_state_path: std::path::Path::new("TODO : path to persistent_state"),
        wal_path: std::path::Path::new("TODO : path to wal"),
    };

    info!("{:?}", raft_config);

    let (log_tx, log_rx) = mpsc::channel(15);
    let (req_tx, req_rx) = mpsc::channel(15);
    if config.raft_mock_flag {
        info!("RUN MOCK RAFT");
        raft::mock_raft::run_mock_raft(raft_config, log_tx, req_rx)
    } else {
        info!("RUN RAFT");
        raft::run_raft(raft_config, log_tx, req_rx)
    };

    // writer task
    let (writer_tx, writer_rx): (
        Sender<(String, data_model::msg::ClientMsg)>,
        Receiver<(String, data_model::msg::ClientMsg)>,
    ) = mpsc::channel(15);

    let writer = events::task::Writer::new(hash.clone());
    writer.start(writer_rx, req_tx).await;

    // publisher task
    let (pub_tx, pub_rx): (
        Sender<(String, SplitSink<WebSocketStream<TcpStream>, Message>)>,
        Receiver<(String, SplitSink<WebSocketStream<TcpStream>, Message>)>,
    ) = mpsc::channel(15);

    let publisher = events::task::Publisher::new(Vec::new(), hash.clone());
    publisher.start(log_rx, pub_rx).await;

    return (writer_tx, pub_tx);
}

#[tokio::main]
async fn main() {
    let config: Config = setup().await;

    info!("{:?}", config);

    run_axum(&config).await;

    // writer and publisher set up
    let client_commit_idx: Arc<tokio::sync::Mutex<HashMap<String, u64>>> =
        Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    let (writer_tx, pub_tx) = run_tasks(client_commit_idx, &config).await;

    // websocket server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.socket_ports[config.self_domain_idx]));
    let server = TcpListener::bind(addr).await;
    let listener = server.expect("failed to bind");

    while let Ok((stream, addr)) = listener.accept().await {
        let w_tx = writer_tx.clone();
        let p_tx = pub_tx.clone();
        tokio::spawn(async move {
            events::handler::client_handler(stream, addr, w_tx, p_tx).await;
        });
    }
}
