use std::collections::HashSet;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};

use super::message::P2pMessage;
use super::peer::{read_peer_loop, write_peer_loop};

pub struct Swarm {
    /// Outgoing message senders to all connected peers
    peers: Arc<RwLock<Vec<mpsc::Sender<P2pMessage>>>>,
    /// Hashes of transactions we already received (prevents broadcast loops)
    seen_transactions: Arc<RwLock<HashSet<String>>>,
    /// Hashes of blocks we already received
    seen_blocks: Arc<RwLock<HashSet<String>>>,
}

impl Swarm {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(Vec::new())),
            seen_transactions: Arc::new(RwLock::new(HashSet::new())),
            seen_blocks: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Returns the number of currently connected peers
    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }

    /// Returns live mempool statistics: (count, total_bytes, estimated_memory_usage)
    pub async fn mempool_info(&self) -> (usize, usize, usize) {
        let txs = self.seen_transactions.read().await;
        let size = txs.len();
        let bytes: usize = txs.iter().map(|hex| hex.len() / 2).sum();// what does this mean?
        let usage = bytes + (size * 64);
        (size, bytes, usage)
    }

    /// Broadcast a message to all currently connected peers
    pub async fn broadcast(&self, msg: P2pMessage) {
        let peers = self.peers.read().await;
        for peer in peers.iter() {
            let _ = peer.send(msg.clone()).await;
        }
    }

    /// Relay a new transaction: marks it as seen and broadcasts it if new
    pub async fn relay_transaction(&self, tx_hex: String) -> bool {
        let mut seen = self.seen_transactions.write().await;
        if seen.insert(tx_hex.clone()) {
            drop(seen);
            self.broadcast(P2pMessage::NewTransaction { tx_hex }).await;
            true
        } else {
            false // Already seen, ignore
        }
    }

    /// Relay a new block: marks it as seen and broadcasts it if new
    pub async fn relay_block(&self, block_hex: String) -> bool {
        let mut seen = self.seen_blocks.write().await;
        if seen.insert(block_hex.clone()) {
            drop(seen);
            self.broadcast(P2pMessage::NewBlock { block_hex }).await;
            true
        } else {
            false // Already seen, ignore
        }
    }

    /// Handle an incoming message from a peer
    pub async fn handle_incoming(
        self: &Arc<Self>,
        msg: P2pMessage,
        peer_sender: &mpsc::Sender<P2pMessage>,
    ) {
        match msg {
            P2pMessage::Ping(nonce) => {
                let _ = peer_sender.send(P2pMessage::Pong(nonce)).await;
            }
            P2pMessage::Pong(_) => {
                // Heartbeat confirmed
            }
            P2pMessage::NewTransaction { tx_hex } => {
                let is_new = self.relay_transaction(tx_hex.clone()).await;
                if is_new {
                    println!("[P2P] Received and gossiped new transaction: {}", tx_hex);
                }
            }
            P2pMessage::NewBlock { block_hex } => {
                let is_new = self.relay_block(block_hex.clone()).await;
                if is_new {
                    println!("[P2P] Received and gossiped new block: {}", block_hex);
                }
            }
            P2pMessage::GetBlocks { from_height } => {
                println!("[P2P] Peer requested blocks from height: {}", from_height);
            }
        }
    }

    /// Connect to an external peer node at `addr` (outbound connection)
    pub async fn connect_to_peer(self: &Arc<Self>, addr: &str) -> Result<(), &'static str> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|_| "Failed to connect to peer")?;

        self.spawn_peer_handler(stream).await;
        Ok(())
    }

    /// Register a new connected TCP stream (inbound or outbound)
    pub async fn spawn_peer_handler(self: &Arc<Self>, stream: TcpStream) {
        let (reader, writer) = stream.into_split();
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<P2pMessage>(100);
        let (incoming_tx, mut incoming_rx) = mpsc::channel::<P2pMessage>(100);

        // Store sender in peer list
        {
            let mut peers = self.peers.write().await;
            peers.push(outgoing_tx.clone());
        }

        // Spawn writer loop
        tokio::spawn(async move {
            let _ = write_peer_loop(writer, outgoing_rx).await;
        });

        // Spawn reader loop
        tokio::spawn(async move {
            let _ = read_peer_loop(reader, incoming_tx).await;
        });

        // Spawn dispatcher loop for incoming messages
        let swarm_clone = self.clone();
        let peer_sender = outgoing_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = incoming_rx.recv().await {
                swarm_clone.handle_incoming(msg, &peer_sender).await;
            }
        });

        // Send initial Ping
        let _ = outgoing_tx.send(P2pMessage::Ping(1)).await;
    }

    /// Start listening for incoming peer connections on `addr`
    pub async fn start_listener(self: &Arc<Self>, addr: &str) -> Result<(), &'static str> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|_| "Failed to bind P2P listener")?;

        println!("[P2P] Swarm listening for peers on {}", addr);

        let swarm = self.clone();
        tokio::spawn(async move {
            while let Ok((socket, peer_addr)) = listener.accept().await {
                println!("[P2P] Accepted inbound connection from {}", peer_addr);
                swarm.spawn_peer_handler(socket).await;
            }
        });

        Ok(())
    }

    /// Automatically connects to seed nodes in the background (like Ethereum bootnodes)
    pub async fn connect_to_seeds(self: &Arc<Self>, seeds: &[&str], my_addr: &str) {
        for &seed in seeds {
            if seed == my_addr {
                continue;
            }
            println!("[P2P] Discovered default bootnode at {}, connecting...", seed);
            if let Ok(()) = self.connect_to_peer(seed).await {
                println!("[P2P] Successfully connected to bootnode: {}", seed);
            }
        }
    }
}

pub const DEFAULT_SEED_NODES: &[&str] = &["127.0.0.1:30303"];

