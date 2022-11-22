use slotmap::DefaultKey;

pub(super) struct Player {
    pub key: DefaultKey,
    pub username: String,
    pub uuid: String,
}
