// src/network.rs
use tokio::sync::mpsc::Sender;
use crate::message::PBFTMessage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use log::debug;

lazy_static::lazy_static! {
    pub static ref NETWORK: Arc<Mutex<HashMap<usize, Sender<PBFTMessage>>>> = Arc::new(Mutex::new(HashMap::new()));
}

pub async fn send_message(node_id: usize, msg: PBFTMessage) {
    let network = NETWORK.lock().unwrap();
    if let Some(sender) = network.get(&node_id) {
        debug!("发送消息到节点{}: {:?}", node_id, msg);
        let _ = sender.send(msg).await;
    } else {
        debug!("节点{}的发送器未注册", node_id);
    }
}

pub fn register_node(node_id: usize, sender: Sender<PBFTMessage>) {
    let mut network = NETWORK.lock().unwrap();
    network.insert(node_id, sender);
    debug!("节点{}已注册到网络中", node_id);
}
