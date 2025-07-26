use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::{DeserializeAs, SerializeAs};

use grammers_tl_types as tl_types;

macro_rules! impl_serialize_as {
    ($remote_type: ty, $local_type: ty) => {
        impl SerializeAs<$remote_type> for $local_type {
            fn serialize_as<S>(value: &$remote_type, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                <$local_type>::serialize(value, serializer)
            }
        }

        impl<'a> DeserializeAs<'a, $remote_type> for $local_type {
            fn deserialize_as<D>(deserializer: D) -> Result<$remote_type, D::Error>
            where
                D: serde::Deserializer<'a>,
            {
                <$local_type>::deserialize(deserializer)
            }
        }
    };
}

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::InputPeerChat")]
struct InputPeerChatDef {
    chat_id: i64,
}

impl_serialize_as!(tl_types::types::InputPeerChat, InputPeerChatDef);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::InputPeerUser")]
struct InputPeerUserDef {
    user_id: i64,
    access_hash: i64,
}

impl_serialize_as!(tl_types::types::InputPeerUser, InputPeerUserDef);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::InputPeerChannel")]
struct InputPeerChannelDef {
    channel_id: i64,
    access_hash: i64,
}

impl_serialize_as!(tl_types::types::InputPeerChannel, InputPeerChannelDef);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::InputPeerUserFromMessage")]
struct InputPeerUserFromMessageDef {
    #[serde_as(as = "InputPeerDef")]
    peer: tl_types::enums::InputPeer,
    msg_id: i32,
    user_id: i64,
}

impl_serialize_as!(
    tl_types::types::InputPeerUserFromMessage,
    InputPeerUserFromMessageDef
);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::InputPeerChannelFromMessage")]
struct InputPeerChannelFromMessageDef {
    #[serde_as(as = "InputPeerDef")]
    peer: tl_types::enums::InputPeer,
    msg_id: i32,
    channel_id: i64,
}

impl_serialize_as!(
    tl_types::types::InputPeerChannelFromMessage,
    InputPeerChannelFromMessageDef
);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::enums::InputPeer")]
enum InputPeerDef {
    Empty,
    PeerSelf,
    Chat(#[serde_as(as = "InputPeerChatDef")] tl_types::types::InputPeerChat),
    User(#[serde_as(as = "InputPeerUserDef")] tl_types::types::InputPeerUser),
    Channel(#[serde_as(as = "InputPeerChannelDef")] tl_types::types::InputPeerChannel),
    UserFromMessage(
        #[serde_as(as = "Box<InputPeerUserFromMessageDef>")]
        Box<tl_types::types::InputPeerUserFromMessage>,
    ),
    ChannelFromMessage(
        #[serde_as(as = "Box<InputPeerChannelFromMessageDef>")]
        Box<tl_types::types::InputPeerChannelFromMessage>,
    ),
}

impl_serialize_as!(tl_types::enums::InputPeer, InputPeerDef);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::DialogFilter")]
struct DialogFilterTypeDef {
    contacts: bool,
    non_contacts: bool,
    groups: bool,
    broadcasts: bool,
    bots: bool,
    exclude_muted: bool,
    exclude_read: bool,
    exclude_archived: bool,
    id: i32,
    title: String,
    emoticon: Option<String>,
    color: Option<i32>,
    #[serde_as(as = "Vec<InputPeerDef>")]
    pinned_peers: Vec<tl_types::enums::InputPeer>,
    #[serde_as(as = "Vec<InputPeerDef>")]
    include_peers: Vec<tl_types::enums::InputPeer>,
    #[serde_as(as = "Vec<InputPeerDef>")]
    exclude_peers: Vec<tl_types::enums::InputPeer>,
}

impl_serialize_as!(tl_types::types::DialogFilter, DialogFilterTypeDef);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::DialogFilterChatlist")]
struct DialogFilterChatlistTypeDef {
    has_my_invites: bool,
    id: i32,
    title: String,
    emoticon: Option<String>,
    color: Option<i32>,
    #[serde_as(as = "Vec<InputPeerDef>")]
    pinned_peers: Vec<tl_types::enums::InputPeer>,
    #[serde_as(as = "Vec<InputPeerDef>")]
    include_peers: Vec<tl_types::enums::InputPeer>,
}

impl_serialize_as!(
    tl_types::types::DialogFilterChatlist,
    DialogFilterChatlistTypeDef
);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::enums::DialogFilter")]
enum DialogFilterDef {
    Filter(#[serde_as(as = "DialogFilterTypeDef")] tl_types::types::DialogFilter),
    Default,
    Chatlist(#[serde_as(as = "DialogFilterChatlistTypeDef")] tl_types::types::DialogFilterChatlist),
}

impl_serialize_as!(tl_types::enums::DialogFilter, DialogFilterDef);

#[serde_as]
#[derive(Deserialize, Serialize)]
#[serde(remote = "tl_types::types::messages::DialogFilters")]
pub struct DialogFiltersDef {
    tags_enabled: bool,
    #[serde_as(as = "Vec<DialogFilterDef>")]
    filters: Vec<tl_types::enums::DialogFilter>,
}
