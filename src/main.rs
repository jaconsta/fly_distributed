use std::io::{StdoutLock, Write};

// use std::collections::HashMap;
use anyhow::{bail, Context};
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
}

// State machines
struct EchoNode {
    id: usize,
}

impl EchoNode {
    pub fn step(&mut self, input: Message, output: &mut StdoutLock) -> anyhow::Result<()> {
        match input.body.payload {
            Payload::Init { .. } => {
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::InitOk,
                    },
                };
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
            Payload::InitOk | Payload::GenerateOk { .. } => bail!("Oks should never happen"),
            _ => {}
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message>();

    let mut state = EchoNode { id: 1 };
    for input in inputs {
        let input = input.context("Message input failed to deserealize")?;
        state.step(input, &mut stdout).context("EchoNode failed")?;
    }

    Ok(())
}
