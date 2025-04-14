#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum SkulkPhase {
    Active,
    Cooldown,
    Inactive,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum ComparitorMode {
    Compare,
    Subtract,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum StructureBlockMode {
    Save,
    Load,
    Corner,
    Data,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum PistonType {
    Normal,
    Sticky,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum ChestType {
    Left,
    Right,
    Single,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum NoteblockInstrument {
    Banjo,
    BaseDrum,
    Bass,
    Bell,
    Bit,
    Chime,
    CowBell,
    Creeper,
    CustomHead,
    Didgeridoo,
    Dragon,
    Flute,
    Guitar,
    Harp,
    Hat,
    IronXylophone,
    Piglin,
    Pling,
    Skeleton,
    Snare,
    WitherSkeleton,
    Xylophone,
    Zombie,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum TrialSpawnerState {
    Active,
    Cooldown,
    EjectingReward,
    Inactive,
    WaitingForPlayers,
    WaitingForRewardEjection,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum RedstoneOrientation {
    None,
    Side,
    Up,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BellAttachment {
    Floor,
    Ceiling,
    SingleWall,
    DoubleWall,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BedPart {
    Head,
    Foot,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum DripstoneThickness {
    TipMerge,
    Tip,
    Frustum,
    Middle,
    Base,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum WallHeight {
    None,
    Low,
    Tall,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BambooLeafSize {
    None,
    Small,
    Large,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BlockDirection {
    Up,
    Down,
    North,
    South,
    East,
    West,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BlockOrientation {
    DownEast,
    DownNorth,
    DownSouth,
    DownWest,

    NorthUp,
    SouthUp,
    EastUp,
    WestUp,

    UpNorth,
    UpSouth,
    UpEast,
    UpWest,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BlockTilt {
    None,
    Unstable,
    Partial,
    Full,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BlockFace {
    Ceiling,
    Floor,
    Wall,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BlockHalf {
    Bottom,
    Top,
    Double,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum HingeSide {
    Left,
    Right,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum RailShape {
    AscendingEast,
    AscendingNorth,
    AscendingSouth,
    AscendingWest,
    EastWest,
    NorthEast,
    NorthSouth,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum StairShape {
    InnerLeft,
    InnerRight,
    OuterLeft,
    OuterRight,
    Straight,
}

include!(concat!(env!("OUT_DIR"), "/blocks.rs"));
