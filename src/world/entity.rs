use uuid::Uuid;
use vek::{
    Vec2,
    Vec3,
};

pub type Entity = (EntityId, EntityData);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(transparent)]
pub struct EntityId(u32);

impl From<EntityId> for i32 {
    fn from(val: EntityId) -> Self {
        val.0.cast_signed()
    }
}

impl From<&EntityId> for i32 {
    fn from(val: &EntityId) -> Self {
        val.0.cast_signed()
    }
}

#[derive(Debug)]
pub struct EntityData {
    pub uuid:     Uuid,
    pub position: Vec3<f64>,
    pub heading:  Vec2<f32>,
    pub head_yaw: f32,

    pub velocity: Vec3<f32>,
}
