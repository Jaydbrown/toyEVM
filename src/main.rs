use std::sync::Arc;
use jsonrpsee::server::Server;
use toyEVM::p2p::{Swarm, DEFAULT_SEED_NODES};
use toyEVM::rpc::{
    MiningRpcServer, MiningRpcServerImpl,
    NodeRpcServer, NodeRpcServerImpl,
    RawTransactionsRpcServer, RawTransactionsRpcServerImpl,
    StateRpcServer, StateRpcServerImpl,
};

async fn bind_p2p_swarm(
    swarm: &Arc<Swarm>,
    preferred_port: u16,
) -> Result<(String, u16), Box<dyn std::error::Error>> {
    let mut port = preferred_port;
    loop {
        let addr = format!("127.0.0.1:{}", port);
        match swarm.start_listener(&addr).await {
            Ok(()) => return Ok((addr, port)),
            Err(_) => {
                port += 1;
                if port > preferred_port + 50 {
                    return Err("Failed to find available P2P port".into());
                }
            }
        }
    }
}

async fn build_rpc_server(
    preferred_port: u16,
) -> Result<(Server, String, u16), Box<dyn std::error::Error>> {
    let mut port = preferred_port;
    loop {
        let addr = format!("127.0.0.1:{}", port);
        match Server::builder().build(&addr).await {
            Ok(server) => return Ok((server, addr, port)),
            Err(_) => {
                port += 1;
                if port > preferred_port + 50 {
                    return Err("Failed to find available RPC port".into());
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        r"
       ==============          ====          ==        ==
             ==               ==  ==          ==      == 
             ==              ========          ==    ==  
             ==             ==      ==          ======   
       ==    ==            ==        ==           ==     
        ======            ==          ==          ==     

             ======  ==      ==  ===    ===
             ==       ==    ==   ====  ====
             ====      ==  ==    == ==== ==
             ==         ====     ==  ==  ==
             ======      ==      ==      ==

           ==   ==   ====   ====    =====
           ===  ==  ==  ==  == ==   ==   
           == = ==  ==  ==  ==  ==  ==== 
           ==  ===  ==  ==  == ==   ==   
           ==   ==   ====   ====    =====

               Welcome to Jay's EVM Node
        A lightweight Ethereum Virtual Machine & Node
"
    );

    // Optional CLI port overrides: cargo run -- [rpc_port] [p2p_port] [extra_peer_addr]
    let args: Vec<String> = std::env::args().collect();
    let preferred_rpc_port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8545);
    let preferred_p2p_port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(30303);
    let manual_peer = args.get(3).map(|s| s.as_str());

    // 0. Initialize Shared Blockchain (with Ethereum Frontier Genesis Block)
    let blockchain = Arc::new(tokio::sync::RwLock::new(
        toyEVM::blockchain::Blockchain::new(),
    ));

    // 1. Initialize P2P Swarm & bind to an available port
    let swarm = Arc::new(Swarm::new());
    let (p2p_addr, _p2p_port) = bind_p2p_swarm(&swarm, preferred_p2p_port).await?;
    println!("[P2P] Swarm listening for peers on {}", p2p_addr);

    // 2. Automatically connect to default seed nodes
    swarm.connect_to_seeds(DEFAULT_SEED_NODES, &p2p_addr).await;

    // Connect to any additional manual peer if provided
    if let Some(peer) = manual_peer {
        println!("[P2P] Connecting to manual peer: {} ...", peer);
        let _ = swarm.connect_to_peer(peer).await;
    }

    // 3. Build & start JSON-RPC Server on an available port
    let (server, rpc_addr, _rpc_port) = build_rpc_server(preferred_rpc_port).await?;

    let mut rpc_module = StateRpcServerImpl.into_rpc();
    rpc_module.merge(RawTransactionsRpcServerImpl.into_rpc())?;
    let node_rpc = NodeRpcServerImpl::new(swarm.clone(), blockchain.clone());
    rpc_module.merge(node_rpc.into_rpc())?;
    let mining_rpc = MiningRpcServerImpl::new(blockchain.clone());
    rpc_module.merge(mining_rpc.into_rpc())?;

    let handle = server.start(rpc_module);
    println!("[RPC] JSON-RPC Server listening on http://{}", rpc_addr);
    println!("\nAvailable RPC Methods:");
    println!("  - eth_call");
    println!("  - eval_bytecode");
    println!("  - decoderawtransaction");
    println!("  - createrawtransaction");
    println!("  - getblockchaininfo");
    println!("  - getnetworkinfo");
    println!("  - getmempoolinfo");
    println!("  - getblocktemplate");
    println!("  - submitblock");
    println!("  - generate\n");
    println!("Node is running! Press Ctrl+C to stop.\n");

    handle.stopped().await;
    Ok(())
}
