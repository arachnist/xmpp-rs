use futures::stream::StreamExt;
use std::env::args;
use std::process::exit;
use std::str::FromStr;
use tokio_xmpp::Client;
use xmpp_parsers::jid::{BareJid, Jid};
use xmpp_parsers::message::{Body, Message, MessageType};
use xmpp_parsers::presence::{Presence, Show as PresenceShow, Type as PresenceType};

#[tokio::main]
async fn main() {
    env_logger::init();

    let args: Vec<String> = args().collect();
    if args.len() != 3 {
        println!("Usage: {} <jid> <password>", args[0]);
        exit(1);
    }
    let jid = BareJid::from_str(&args[1]).expect(&format!("Invalid JID: {}", &args[1]));
    let password = &args[2];

    // Client instance
    let mut client = Client::new(jid, password.to_owned());

    // Main loop, processes events
    while let Some(event) = client.next().await {
        println!("event: {:?}", event);
        if event.is_online() {
            let jid = event
                .get_jid()
                .map(|jid| format!("{}", jid))
                .unwrap_or("unknown".to_owned());
            println!("Online at {}", jid);

            let presence = make_presence();
            client.send_stanza(presence.into()).await.unwrap();
        } else if let Some(message) = event
            .into_stanza()
            .and_then(|stanza| Message::try_from(stanza).ok())
        {
            match (message.from, message.bodies.get("")) {
                (Some(ref from), Some(ref body)) if body.0 == "die" => {
                    println!("Secret die command triggered by {}", from);
                    break;
                }
                (Some(ref from), Some(ref body)) => {
                    if message.type_ != MessageType::Error {
                        // This is a message we'll echo
                        let reply = make_reply(from.clone(), &body.0);
                        client.send_stanza(reply.into()).await.unwrap();
                    }
                }
                _ => {}
            }
        }
    }

    client.send_end().await.unwrap();
}

// Construct a <presence/>
fn make_presence() -> Presence {
    let mut presence = Presence::new(PresenceType::None);
    presence.show = Some(PresenceShow::Chat);
    presence
        .statuses
        .insert(String::from("en"), String::from("Echoing messages."));
    presence
}

// Construct a chat <message/>
fn make_reply(to: Jid, body: &str) -> Message {
    let mut message = Message::new(Some(to));
    message.bodies.insert(String::new(), Body(body.to_owned()));
    message
}
