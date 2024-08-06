//! Low-level stream establishment

mod xmpp_codec;
mod xmpp_stream;

pub use xmpp_codec::{Packet, XmppCodec};
pub(crate) use xmpp_stream::add_stanza_id;
pub use xmpp_stream::XmppStream;
