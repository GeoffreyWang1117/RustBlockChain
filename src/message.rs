// src/message.rs

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PBFTMessage {
    Request {
        operation: String,
    },
    PrePrepare {
        view: u64,
        sequence_number: u64,
        digest: String,
    },
    Prepare {
        view: u64,
        sequence_number: u64,
        digest: String,
        sender_id: usize, // Added sender_id field
    },
    Commit {
        view: u64,
        sequence_number: u64,
        digest: String,
    },
    ViewChange {
        view: u64,
        last_sequence_number: u64,
        node_id: usize, // Added node_id field
    },
    NewView {
        view: u64,
        view_change_messages: Vec<PBFTMessage>, // Added view_change_messages field
    },
    PubKey {
        node_id: usize,
        public_key: Vec<u8>,
    },
    SignedMessage {
        message: Box<PBFTMessage>,
        signature: Vec<u8>,
        sender_id: usize,
    },
    ByzantineVote {
        suspected_id: usize,
        sender_id: usize,
    },
}
