// src/main.rs

mod config;
mod message;
mod network;
mod node;

use crate::node::Node;
use crate::network::register_node;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use crate::node::NodeState;
use log::info;
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use std::collections::HashMap;

fn parse_args() -> (usize, bool) {
    let args: Vec<String> = std::env::args().collect();
    let node_id: usize = args.get(1).unwrap_or(&"0".to_string()).parse().unwrap();
    let is_byzantine = args.get(2).map_or(false, |s| s == "byzantine");
    (node_id, is_byzantine)
}

#[tokio::main]
async fn main() {
    println!("Node started");
    // Parse command-line arguments
    let (node_id, is_byzantine) = parse_args();

    // Initialize logger
    init_logger(node_id);

    info!("启动节点{}，是否为拜占庭节点: {}", node_id, is_byzantine);

    // Create communication channel
    let (tx, rx) = mpsc::channel(100);
    register_node(node_id, tx.clone());

    // Initialize node state
    let _node_state = Arc::new(Mutex::new(NodeState::load(node_id)));

    // Generate keypair
    let mut csprng = OsRng;
    let keypair = Keypair::generate(&mut csprng);

    // Collect public keys (in practice, exchange over the network)
    let mut public_keys = HashMap::new();
    public_keys.insert(node_id, keypair.public);

    // Create node instance
    let mut node = Node::new(
        node_id,
        0,
        keypair,
        public_keys,
        rx,
        is_byzantine,
    );

    // If primary node, simulate client request
    if node.is_primary() {
        info!("节点{}是主节点，模拟发送客户端请求", node_id);
        let request = crate::message::PBFTMessage::Request {
            operation: format!("操作{}", node.sequence_number + 1),
        };
        node.handle_request(request).await;
    } else {
        info!("节点{}是副本节点，等待消息", node_id);
    }

    // Run node
    node.run().await;
}

fn init_logger(node_id: usize) {
    use std::fs::File;
    use std::io::Write;
    use chrono::Local;
    use env_logger::Builder;
    use log::LevelFilter;

    let log_file = format!("node_{}.log", node_id);
    let file = File::create(log_file).unwrap();

    Builder::new()
        .format(move |_buf, record| {
            writeln!(
                &mut file.try_clone().unwrap(),
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            ).unwrap();
            Ok(())
        })
        .filter(None, LevelFilter::Info)
        .init();
}
