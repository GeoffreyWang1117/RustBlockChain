// src/node.rs

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Receiver;
use tokio::time::{sleep, Duration, Instant};
use tokio::select;
use crate::message::PBFTMessage;
use crate::network::send_message;
use crate::config::{F, N};
use log::{info, error, debug};
use ed25519_dalek::{Keypair, Signature, Signer, Verifier, PublicKey};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct NodeState {
    pub prepared: HashSet<(u64, String)>,
    pub committed: HashSet<(u64, String)>,
    pub messages: Vec<PBFTMessage>,
    pub view_change_messages: Vec<PBFTMessage>,
    pub byzantine_votes: HashMap<usize, HashSet<usize>>,
}

impl NodeState {
    pub fn save(&self, node_id: usize) {
        let filename = format!("node_{}_state.json", node_id);
        let data = serde_json::to_string(self).unwrap();
        std::fs::write(filename, data).unwrap();
    }

    pub fn load(node_id: usize) -> Self {
        let filename = format!("node_{}_state.json", node_id);
        if let Ok(data) = std::fs::read_to_string(filename) {
            serde_json::from_str(&data).unwrap()
        } else {
            NodeState {
                prepared: HashSet::new(),
                committed: HashSet::new(),
                messages: Vec::new(),
                view_change_messages: Vec::new(),
                byzantine_votes: HashMap::new(),
            }
        }
    }
}

pub struct Node {
    pub id: usize,
    pub view: u64,
    pub sequence_number: u64,
    pub digest: String,
    pub state: Arc<Mutex<NodeState>>,
    pub receiver: Receiver<PBFTMessage>,
    pub timeout_duration: Duration,
    pub last_message_time: Instant,
    pub view_change_in_progress: bool,
    pub keypair: Keypair,
    pub public_keys: HashMap<usize, PublicKey>,
    pub is_byzantine: bool,
    pub suspected_nodes: HashSet<usize>,
    pub blacklist: HashSet<usize>,
    pub pending_requests: Vec<PBFTMessage>,
    pub new_view_timer: Option<tokio::task::JoinHandle<()>>,
}

impl Node {
    pub fn new(
        id: usize,
        view: u64,
        keypair: Keypair,
        public_keys: HashMap<usize, PublicKey>,
        receiver: Receiver<PBFTMessage>,
        is_byzantine: bool,
    ) -> Self {
        Node {
            id,
            view,
            sequence_number: 0,
            digest: String::new(),
            state: Arc::new(Mutex::new(NodeState::load(id))),
            receiver,
            timeout_duration: Duration::from_secs(5),
            last_message_time: Instant::now(),
            view_change_in_progress: false,
            keypair,
            public_keys,
            is_byzantine,
            suspected_nodes: HashSet::new(),
            blacklist: HashSet::new(),
            pending_requests: Vec::new(),
            new_view_timer: None,
        }
    }

    pub async fn run(&mut self) {
        info!("节点{}开始运行", self.id);

        // 广播公钥
        let pubkey_msg = PBFTMessage::PubKey {
            node_id: self.id,
            public_key: self.keypair.public.to_bytes().to_vec(),
        };
        self.broadcast(&pubkey_msg).await;

        loop {
            let timeout = sleep(self.timeout_duration);
            tokio::pin!(timeout);

            select! {
                Some(msg) = self.receiver.recv() => {
                    self.last_message_time = Instant::now();
                    self.handle_message(msg).await;
                }
                () = &mut timeout => {
                    self.handle_timeout().await;
                }
            }
        }
    }

    pub async fn handle_message(&mut self, msg: PBFTMessage) {
        let mut message_queue = vec![msg];

        while let Some(current_msg) = message_queue.pop() {
            // 检查发送者是否在黑名单中
            let sender_id = match &current_msg {
                PBFTMessage::SignedMessage { sender_id, .. } => *sender_id,
                PBFTMessage::ByzantineVote { sender_id, .. } => *sender_id,
                PBFTMessage::PubKey { node_id, .. } => *node_id,
                _ => self.id, // 自己发送的消息
            };

            if self.blacklist.contains(&sender_id) {
                info!("节点{}忽略来自拜占庭节点{}的消息", self.id, sender_id);
                continue;
            }

            debug!("节点{}收到消息: {:?}", self.id, current_msg);
            match current_msg {
                PBFTMessage::SignedMessage { message, signature, sender_id } => {
                    // 验证签名
                    if let Some(pubkey) = self.public_keys.get(&sender_id) {
                        let message_bytes = serde_json::to_vec(&message).unwrap();
                        let signature = Signature::from_bytes(&signature).unwrap();

                        if pubkey.verify(&message_bytes, &signature).is_ok() {
                            debug!("节点{}验证签名成功，来自节点{}", self.id, sender_id);
                            // 将内部消息加入队列
                            message_queue.push(*message);
                        } else {
                            error!("节点{}验证签名失败，来自节点{}", self.id, sender_id);
                        }
                    } else {
                        error!("节点{}没有节点{}的公钥，无法验证签名", self.id, sender_id);
                    }
                }
                _ => {
                    // 调用相应的处理函数
                    self.process_message(current_msg).await;
                }
            }
        }
    }

    async fn process_message(&mut self, msg: PBFTMessage) {
        match msg {
            PBFTMessage::PrePrepare { .. } => {
                self.handle_preprepare(msg).await;
            }
            PBFTMessage::Prepare { .. } => {
                self.handle_prepare(msg).await;
            }
            PBFTMessage::Commit { .. } => {
                self.handle_commit(msg).await;
            }
            PBFTMessage::ViewChange { .. } => {
                self.handle_view_change(msg).await;
            }
            PBFTMessage::NewView { .. } => {
                self.handle_new_view(msg).await;
            }
            PBFTMessage::ByzantineVote { suspected_id, sender_id } => {
                self.handle_byzantine_vote(suspected_id, sender_id).await;
            }
            PBFTMessage::PubKey { node_id, public_key } => {
                // 处理公钥消息
                let pubkey = PublicKey::from_bytes(&public_key).unwrap();
                self.public_keys.insert(node_id, pubkey);
                info!("节点{}收到节点{}的公钥", self.id, node_id);
            }
            PBFTMessage::Request { .. } => {
                self.handle_request(msg).await;
            }
            _ => {
                debug!("节点{}收到未处理的消息类型: {:?}", self.id, msg);
            }
        }
    }

    pub async fn handle_request(&mut self, msg: PBFTMessage) {
        if let PBFTMessage::Request { operation } = msg.clone() {
            // 将请求加入待处理队列
            self.pending_requests.push(msg.clone());

            if self.is_primary() && !self.view_change_in_progress {
                info!("节点{}（主节点）处理客户端请求: {}", self.id, operation);
                self.sequence_number += 1;
                let digest = self.compute_digest(&operation);
                self.digest = digest.clone();

                let preprepare_msg = PBFTMessage::PrePrepare {
                    view: self.view,
                    sequence_number: self.sequence_number,
                    digest: digest.clone(),
                };

                debug!("节点{}广播PrePrepare消息: {:?}", self.id, preprepare_msg);
                self.broadcast(&preprepare_msg).await;
            } else {
                info!("节点{}不是主节点，等待主节点处理请求", self.id);
            }
        }
    }

    async fn handle_preprepare(&mut self, msg: PBFTMessage) {
        if let PBFTMessage::PrePrepare { view, sequence_number, digest } = msg.clone() {
            info!("节点{}处理PrePrepare消息: view={}, seq={}, digest={}", self.id, view, sequence_number, digest);

            if view == self.view && !self.is_primary() {
                self.sequence_number = sequence_number;
                self.digest = digest.clone();

                let prepare_digest = if self.is_byzantine {
                    // 拜占庭节点发送错误的摘要
                    let wrong_digest = "错误的摘要".to_string();
                    info!("拜占庭节点{}发送错误的Prepare摘要", self.id);
                    wrong_digest
                } else {
                    digest.clone()
                };

                let prepare_msg = PBFTMessage::Prepare {
                    view,
                    sequence_number,
                    digest: prepare_digest,
                    sender_id: self.id,
                };

                debug!("节点{}广播Prepare消息: {:?}", self.id, prepare_msg);
                self.broadcast(&prepare_msg).await;
            } else {
                debug!("节点{}收到的PrePrepare消息视图不匹配或自身为主节点，忽略", self.id);
            }
        }
    }

    async fn handle_prepare(&mut self, msg: PBFTMessage) {
        info!("节点{}处理Prepare消息: {:?}", self.id, msg);

        let mut state = self.state.lock().unwrap();
        state.messages.push(msg.clone());

        // 收集不同节点发送的摘要
        let mut digest_counts: HashMap<String, HashSet<usize>> = HashMap::new();
        for m in &state.messages {
            if let PBFTMessage::Prepare { view, sequence_number, digest, .. } = m {
                if *view == self.view && *sequence_number == self.sequence_number {
                    digest_counts.entry(digest.clone()).or_insert_with(HashSet::new).insert(self.id);
                }
            }
        }

        // 检测是否存在不一致的摘要
        if digest_counts.len() > 1 {
            info!("节点{}检测到摘要不一致，可能存在拜占庭节点", self.id);
            let messages = state.messages.clone(); // 克隆消息列表
            drop(state); // 释放锁
            self.detect_byzantine_nodes(&messages).await;
        } else {
            drop(state); // 释放锁
        }

        // 找到收到最多的摘要
        let max_count = digest_counts.values().map(|s| s.len()).max().unwrap_or(0);
        if max_count >= 2 * F {
            // 找到正确的摘要
            let correct_digest = digest_counts.iter().find(|(_, s)| s.len() == max_count).unwrap().0.clone();

            let mut state = self.state.lock().unwrap();
            if !state.prepared.contains(&(self.sequence_number, correct_digest.clone())) {
                state.prepared.insert((self.sequence_number, correct_digest.clone()));
                state.save(self.id);
                info!("节点{}进入Prepared状态，序列号: {}", self.id, self.sequence_number);

                let commit_msg = PBFTMessage::Commit {
                    view: self.view,
                    sequence_number: self.sequence_number,
                    digest: correct_digest,
                };

                debug!("节点{}广播Commit消息: {:?}", self.id, commit_msg);
                self.broadcast(&commit_msg).await;
            }
        }
    }

    async fn detect_byzantine_nodes(&mut self, messages: &Vec<PBFTMessage>) {
        let mut digest_map: HashMap<String, HashSet<usize>> = HashMap::new();

        for m in messages {
            if let PBFTMessage::Prepare { digest, sender_id, .. } = m {
                digest_map.entry(digest.clone()).or_insert_with(HashSet::new).insert(*sender_id);
            }
        }

        // 假设正确的摘要是收到最多的那个
        let correct_digest = digest_map.iter().max_by_key(|&(_, senders)| senders.len()).unwrap().0.clone();

        for (digest, senders) in digest_map {
            if digest != correct_digest {
                for sender_id in senders {
                    self.suspected_nodes.insert(sender_id);
                    info!("节点{}将节点{}标记为可疑", self.id, sender_id);

                    // 广播投票消息
                    let vote_msg = PBFTMessage::ByzantineVote {
                        suspected_id: sender_id,
                        sender_id: self.id,
                    };
                    self.broadcast(&vote_msg).await;
                }
            }
        }
    }

    async fn handle_byzantine_vote(&mut self, suspected_id: usize, sender_id: usize) {
        info!("节点{}收到来自节点{}的拜占庭投票，怀疑节点{}", self.id, sender_id, suspected_id);

        let mut state = self.state.lock().unwrap();
        let entry = state.byzantine_votes.entry(suspected_id).or_insert_with(HashSet::new);
        entry.insert(sender_id);

        if entry.len() >= 2 * F + 1 {
            self.blacklist.insert(suspected_id);
            info!("节点{}确定节点{}为拜占庭节点，将其加入黑名单", self.id, suspected_id);
        }
    }

    async fn handle_commit(&mut self, msg: PBFTMessage) {
        info!("节点{}处理Commit消息: {:?}", self.id, msg);

        // 收集Commit消息
        let mut state = self.state.lock().unwrap();
        state.messages.push(msg.clone());

        let commit_count = state.messages.iter().filter(|m| {
            if let PBFTMessage::Commit { view, sequence_number, digest } = m {
                *view == self.view && *sequence_number == self.sequence_number && *digest == self.digest
            } else {
                false
            }
        }).count();

        debug!("节点{}收到的匹配的Commit消息数量: {}", self.id, commit_count);

        if commit_count >= 2 * F + 1 {
            if !state.committed.contains(&(self.sequence_number, self.digest.clone())) {
                state.committed.insert((self.sequence_number, self.digest.clone()));
                state.save(self.id);
                info!("节点{}已提交请求，序列号: {}", self.id, self.sequence_number);
                // 执行操作或回复客户端
            }
        }
    }

    async fn handle_timeout(&mut self) {
        if Instant::now().duration_since(self.last_message_time) >= self.timeout_duration {
            if !self.view_change_in_progress {
                info!("节点{}检测到超时，触发视图切换", self.id);
                self.start_view_change().await;
            }
        }
    }

    async fn start_view_change(&mut self) {
        self.view_change_in_progress = true;
        self.view += 1;
        self.sequence_number = 0;
        self.digest.clear();

        let view_change_msg = PBFTMessage::ViewChange {
            view: self.view,
            last_sequence_number: self.sequence_number,
            node_id: self.id,
        };

        self.broadcast(&view_change_msg).await;
        self.state.lock().unwrap().view_change_messages.push(view_change_msg.clone());

        // 启动新视图定时器
        let timeout_duration = self.timeout_duration;
        let node_id = self.id;
        let view = self.view;
        self.new_view_timer = Some(tokio::spawn(async move {
            tokio::time::sleep(timeout_duration).await;
            info!("节点{}的新视图定时器超时，视图{}", node_id, view);
            // 可以在这里处理新视图超时逻辑
        }));
    }

    async fn handle_view_change(&mut self, msg: PBFTMessage) {
        if let PBFTMessage::ViewChange { view, node_id, .. } = msg {
            if view == self.view {
                info!("节点{}收到来自节点{}的ViewChange消息，视图{}", self.id, node_id, view);
                self.state.lock().unwrap().view_change_messages.push(msg.clone());

                let count = self.state.lock().unwrap().view_change_messages.iter().filter(|m| {
                    if let PBFTMessage::ViewChange { view: v, .. } = m {
                        *v == self.view
                    } else {
                        false
                    }
                }).count();

                if count >= 2 * F && self.is_primary() {
                    // 作为新主节点，发送NewView消息
                    self.send_new_view().await;
                }
            }
        }
    }

    async fn send_new_view(&mut self) {
        let view_change_messages = self.state.lock().unwrap().view_change_messages.clone();
        let new_view_msg = PBFTMessage::NewView {
            view: self.view,
            view_change_messages,
        };

        info!("新主节点{}发送NewView消息，视图{}", self.id, self.view);
        self.broadcast(&new_view_msg).await;

        // 取消新视图定时器
        if let Some(handle) = &self.new_view_timer {
            handle.abort();
            self.new_view_timer = None;
        }

        self.view_change_in_progress = false;
    }

    async fn handle_new_view(&mut self, msg: PBFTMessage) {
        if let PBFTMessage::NewView { view, .. } = msg {
            if view >= self.view {
                info!("节点{}收到NewView消息，切换到视图{}", self.id, view);
                self.view = view;
                self.view_change_in_progress = false;
                self.sequence_number = 0;
                self.digest.clear();
                self.state.lock().unwrap().view_change_messages.clear();

                // 取消新视图定时器
                if let Some(handle) = &self.new_view_timer {
                    handle.abort();
                    self.new_view_timer = None;
                }

                // 处理从ViewChange消息中恢复的状态（简化处理）

                // 如果自己是新主节点，且有未处理的请求，可以重新发起请求
                if self.is_primary() && !self.pending_requests.is_empty() {
                    let pending_requests = self.pending_requests.clone();
                    for request in pending_requests {
                        self.handle_request(request).await;
                    }
                }
            }
        }
    }

    async fn broadcast(&self, msg: &PBFTMessage) {
        // 更新消息的视图编号
        let msg_with_view = match msg {
            PBFTMessage::PrePrepare { sequence_number, digest, .. } => {
                PBFTMessage::PrePrepare {
                    view: self.view,
                    sequence_number: *sequence_number,
                    digest: digest.clone(),
                }
            }
            PBFTMessage::Prepare { sequence_number, digest, sender_id, .. } => {
                PBFTMessage::Prepare {
                    view: self.view,
                    sequence_number: *sequence_number,
                    digest: digest.clone(),
                    sender_id: *sender_id,
                }
            }
            PBFTMessage::Commit { sequence_number, digest, .. } => {
                PBFTMessage::Commit {
                    view: self.view,
                    sequence_number: *sequence_number,
                    digest: digest.clone(),
                }
            }
            _ => msg.clone(),
        };

        // 对消息进行签名
        let message_bytes = serde_json::to_vec(&msg_with_view).unwrap();
        let signature = self.keypair.sign(&message_bytes);

        let signed_msg = PBFTMessage::SignedMessage {
            message: Box::new(msg_with_view),
            signature: signature.to_bytes().to_vec(),
            sender_id: self.id,
        };

        for i in 0..N {
            if i != self.id {
                debug!("节点{}向节点{}发送签名消息", self.id, i);
                send_message(i, signed_msg.clone()).await;
            }
        }
    }

    pub fn is_primary(&self) -> bool {
        self.id == (self.view as usize % N)
    }

    fn compute_digest(&self, operation: &str) -> String {
        // 使用SHA-256计算摘要
        let digest = ring::digest::digest(&ring::digest::SHA256, operation.as_bytes());
        let hex_digest = hex::encode(digest.as_ref());
        debug!("节点{}计算操作'{}'的摘要: {}", self.id, operation, hex_digest);
        hex_digest
    }
}
