#[repr(u8)]
pub enum Shape {
    None,
    Stair,
    Slab,
    Full,
}

#[repr(u8)]
pub enum TextureMappingProfile {
    AllSides,       // 1 текстура
    AxisAligned,    // 2 текстуры
    TopBottomSides, // 3 текстуры
    OrientedFront,  // 2 текстуры
}

#[repr(u8)]
pub enum Facing {
    Up,
    Bottom,
    North,
    South,
    West,
    East,
}

#[repr(u8)]
pub enum StairFacing {
    VoidTopNorth,
    VoidTopSouth,
    VoidTopWest,
    VoidTopEast,

    VoidBottomNorth,
    VoidBottomSouth,
    VoidBottomWest,
    VoidBottomEast,

    VoidTopNorthWest,
    VoidTopNortEast,
    VoidTopSouthWest,
    VoidTopSouthEast,

    VoidBottomNorthWest,
    VoidBottomNortEast,
    VoidBottomSouthWest,
    VoidBottomSouthEast,
}
