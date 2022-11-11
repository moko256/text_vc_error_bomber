use std::collections::HashMap;
use std::error::Error;
use std::hash::Hash;

/// The model impl.
pub struct Model<GuildId, ChannelId, UserId>
where
    GuildId: Eq + Hash,
    ChannelId: Eq + Hash,
    UserId: Eq + Hash,
{
    pub channel_names: HashMap<(GuildId, ChannelId), String>,
    pub user_vc_pairs: HashMap<UserId, (GuildId, ChannelId)>,
}

impl<GuildId, ChannelId, UserId> Model<GuildId, ChannelId, UserId>
where
    GuildId: Eq + Hash,
    ChannelId: Eq + Hash,
    UserId: Eq + Hash,
{
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Model::<GuildId, ChannelId, UserId> {
            channel_names: HashMap::new(),
            user_vc_pairs: HashMap::new(),
        }
    }

    pub fn add_channel_name_pair(
        &mut self,
        guild_id: GuildId,
        channel_id: ChannelId,
        channel_name: String,
    ) {
        self.channel_names
            .insert((guild_id, channel_id), channel_name);
    }

    pub fn remove_channel_name_pair(&mut self, guild_id: GuildId, channel_id: ChannelId) {
        self.channel_names.remove(&(guild_id, channel_id));
    }

    pub fn add_or_update_user_voice_status(
        &mut self,
        user_id: UserId,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) {
        self.user_vc_pairs.insert(user_id, (guild_id, channel_id));
    }

    pub fn remove_user_voice_status(&mut self, user_id: &UserId) {
        self.user_vc_pairs.remove(&user_id);
    }

    pub fn msg_is_out_of_vc(
        &self,
        msg_user_id: &UserId,
        msg_guild_id: GuildId,
        msg_ch_id: ChannelId,
    ) -> bool {
        if let Some(user_vc_ch_key) = self.user_vc_pairs.get(msg_user_id) {
            self.channel_names.get(user_vc_ch_key)
                != self.channel_names.get(&(msg_guild_id, msg_ch_id))
        } else {
            false
        }
    }

    pub async fn add_or_update_channel_association(
        &mut self,
        vc_ch_name: String,
        text_ch_name: String,
    ) -> Result<(), ChannelAssocError> {
        Ok(())
    }

    pub async fn remove_channel_association(
        &mut self,
        vc_ch_name: String,
        text_ch_name: String,
    ) -> Result<(), ChannelDeAssocError> {
        Ok(())
    }

    pub fn clear(&mut self) {
        self.channel_names.clear();
        self.user_vc_pairs.clear();
    }
}

enum ChannelAssocError {
    UnknownError { error: Box<dyn Error> },
}

enum ChannelDeAssocError {
    PairNotFound {
        vc_ch_name: String,
        text_ch_name: String,
    },
    UnknownError {
        error: Box<dyn Error>,
    },
}

trait AssocRepositoty<GuildId> {
    fn add_or_update(guild_id: GuildId, vc_ch_name: String, text_ch_name: String);
    fn remove(
        guild_id: GuildId,
        vc_ch_name: String,
        text_ch_name: String,
    ) -> Result<(), ChannelDeAssocError>;
}

#[test]
fn normal_case_1() {
    let mut m = Model::<i32, i32, i32>::new();
    m.add_channel_name_pair(0, 1, "VC1".to_string()); // VC
    m.add_channel_name_pair(0, 2, "VC1".to_string()); // Text

    m.add_or_update_user_voice_status(2, 0, 1);

    // User is in VC, and write text channel for VC.
    assert_eq!(m.msg_is_out_of_vc(&2, 0, 2), false);
    // User is in VC, but write text channel that is not for VC.
    assert_eq!(m.msg_is_out_of_vc(&2, 0, 3), true);
    // User is not in VC, and write text channel for VC.
    assert_eq!(m.msg_is_out_of_vc(&3, 0, 2), false);
    // User is not in VC, and write text channel that is not for VC.
    assert_eq!(m.msg_is_out_of_vc(&3, 0, 3), false);
}
