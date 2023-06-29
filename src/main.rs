use std::{
    collections::{HashMap, HashSet},
    io::{StdoutLock, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    src: String,
    dest: String,
    body: MessageBody,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MessageBody {
    msg_id: Option<usize>,
    in_reply_to: Option<usize>,
    #[serde(flatten)]
    payload: Payload,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum Payload {
    Init {
        node_id: String,
        node_ids: Vec<String>,
    },
    InitOk,
    Error {
        code: usize,
        text: String,
    },
    Echo {
        echo: String,
    },
    EchoOk {
        echo: String,
    },
    Generate,
    GenerateOk {
        #[serde(rename = "id")]
        unq_id: String,
    },
    Broadcast {
        message: usize,
    },
    BroadcastOk,
    Read,
    ReadOk {
        messages: Vec<usize>,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
    GossipBroadcast {
        message: Gossiped,
    },
}

// State machines
struct EchoNode {
    id: usize,
}
type Gossiped = HashSet<usize>;
#[derive(Default, Clone)]
struct BroadcastStore {
    messages: Arc<Mutex<Gossiped>>,
    whoami: Arc<Mutex<String>>,
    topology: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl EchoNode {
    pub fn step(
        &mut self,
        input: Message,
        output: &mut StdoutLock,
        broadcast_store: &mut BroadcastStore,
    ) -> anyhow::Result<()> {
        match input.body.payload {
            Payload::Init { .. } => {
                let reply = Message {
                    src: input.dest.clone(),
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::InitOk,
                    },
                };
                let mut whoamit = broadcast_store.whoami.lock().unwrap();
                whoamit.extend(input.dest.clone().chars());
                // Dereference `output` so it can be re-borrowed.
                serde_json::to_writer(&mut *output, &reply).context("Serialize Init response")?;
                output.write_all(b"\n").context("trailing new line")?;
                self.id += 1;
            }
            Payload::Echo { echo } => {
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::EchoOk { echo },
                    },
                };
                serde_json::to_writer(&mut *output, &reply).context("Serialize Echo response")?;
                output.write_all(b"\n").context("trailing new line")?;
                self.id += 1;
            }
            Payload::Generate { .. } => {
                let unique_id = Ulid::new();
                let unique_id = unique_id.to_string();
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::GenerateOk { unq_id: unique_id },
                    },
                };
                serde_json::to_writer(&mut *output, &reply).context("Serialize Echo response")?;
                output.write_all(b"\n").context("trailing new line")?;
                self.id += 1;
            }
            Payload::Broadcast { message } => {
                let broad_store = broadcast_store.clone();
                let mut broad_msg = broad_store.messages.lock().unwrap();
                broad_msg.insert(message.clone());
                let reply = Message {
                    src: input.dest.clone(),
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::BroadcastOk,
                    },
                };

                serde_json::to_writer(&mut *output, &reply).context("Serialize Init response")?;
                output.write_all(b"\n").context("trailing new line")?;
                self.id += 1;
            }
            Payload::Read => {
                let broad_store = broadcast_store.clone();
                let broad_msg = broad_store.messages.lock().unwrap();

                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::ReadOk {
                            messages: broad_msg.clone().into_iter().collect(),
                        },
                    },
                };
                serde_json::to_writer(&mut *output, &reply).context("Serialize Init response")?;
                output.write_all(b"\n").context("trailing new line")?;
                self.id += 1;
            }
            Payload::Topology { topology } => {
                let brc_str = broadcast_store.topology.clone();
                let mut neighbors = brc_str.lock().unwrap();
                neighbors.extend(topology);
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::TopologyOk,
                    },
                };
                serde_json::to_writer(&mut *output, &reply).context("Serialize Init response")?;
                output.write_all(b"\n").context("trailing new line")?;
                self.id += 1;
            }
            Payload::GossipBroadcast { message } => {
                let broad_store = broadcast_store.clone();
                let mut broad_msg = broad_store.messages.lock().unwrap();
                (&mut broad_msg).extend(message);
            }
            Payload::InitOk
            | Payload::GenerateOk { .. }
            | Payload::BroadcastOk { .. }
            | Payload::ReadOk { .. }
            | Payload::TopologyOk => {}
            _ => {}
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin().lock();

    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message>();

    let mut state = EchoNode { id: 1 };
    let mut broadcast_store = BroadcastStore::default();

    let broadcast_thread = broadcast_store.clone();
    std::thread::spawn(move || -> anyhow::Result<()> {
        let mut moreids: usize = 1000;
        loop {
            {
                let src;
                let msgs;
                {
                    let xsrc = broadcast_thread.whoami.clone().lock().unwrap().to_string();
                    let xmsgs = broadcast_thread.messages.lock().unwrap();
                    src = xsrc.clone();
                    msgs = xmsgs.clone();
                }
                let broad_neighbors = broadcast_thread.topology.clone();
                let neighbors = broad_neighbors.lock().unwrap();
                let mut neighbors = neighbors.values().flatten().collect::<Vec<&String>>();
                neighbors.sort();
                neighbors.dedup();
                neighbors.retain(|&neighbor| neighbor != &src);
                for neighbor in neighbors.into_iter() {
                    let reply = Message {
                        src: src.clone(),
                        dest: String::from(neighbor),
                        body: MessageBody {
                            msg_id: Some(moreids),
                            in_reply_to: None,
                            payload: Payload::GossipBroadcast {
                                message: msgs.clone(),
                            },
                        },
                    };

                    let mut output = std::io::stdout().lock();
                    serde_json::to_writer(&mut output, &reply)
                        .context("Serialize Init response")?;
                    output.write_all(b"\n").context("trailing new line")?;
                    moreids += 1;
                }
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    });

    for input in inputs {
        let input = input.context("Message input failed to deserealize")?;

        let mut stdout = std::io::stdout().lock();
        state
            .step(input, &mut stdout, &mut broadcast_store)
            .context("EchoNode failed")?;
    }

    Ok(())
}
