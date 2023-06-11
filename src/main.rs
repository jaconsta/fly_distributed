use std::io::{StdoutLock, Write};

// use std::collections::HashMap;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    src: String,
    dest: String,
    body: MessageBody,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MessageBody {
    // #[serde(rename = "type")]
    // msg_type: String,
    msg_id: Option<usize>,
    in_reply_to: Option<usize>,
    // // node Initialization
    // node_id: Option<String>,
    // node_ids: Option<Vec<String>>,
    // // errors
    // code: Option<usize>,
    // text: Option<String>,
    // // Others
    #[serde(flatten)]
    payload: Payload,
    // rest: HashMap<String, serde_json::Value>,
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
}

// State machines
struct EchoNode {
    id: usize,
}

impl EchoNode {
    pub fn step(
        &mut self,
        input: Message,
        output: &mut StdoutLock, // serde_json::Serializer<StdoutLock>,
    ) -> anyhow::Result<()> {
        match input.body.payload {
            Payload::Init { .. } => {
                // { node_id, node_ids }
                let reply = Message {
                    src: input.dest,
                    dest: input.src,
                    body: MessageBody {
                        msg_id: Some(self.id),
                        in_reply_to: input.body.msg_id,
                        payload: Payload::InitOk,
                    },
                };
                // Dereference Output so it can be re-borrowed.
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
                // // This method does not work because of writting
                // // to buffer does not flush the input and add
                // // a nee line
                // reply
                //     .serialize(output)
                //     .context("Failed to response to echo")?;
                serde_json::to_writer(&mut *output, &reply).context("Serialize Echo response")?;
                output.write_all(b"\n").context("trailing new line")?;
                self.id += 1;
            }
            Payload::InitOk { .. } => bail!("InitOk should never happen"),
            _ => {} // Payload::EchoOk { .. } => {},
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    let inputs = serde_json::Deserializer::from_reader(stdin).into_iter::<Message>();
    // let mut output = serde_json::Serializer::new(stdout);

    let mut state = EchoNode { id: 1 };
    for input in inputs {
        let input = input.context("Message input failed to deserealize")?;
        state.step(input, &mut stdout).context("EchoNode failed")?;
    }

    Ok(())
}
