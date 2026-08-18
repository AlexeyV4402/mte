#[repr(u8)]
pub enum Shape {
    None,
    Stair,
    Slab,
    Full,
}

#[repr(u8)]
pub enum TextureMappingProfile {
    AllSides = 0,       // 1 текстура: абсолютно одинаковая со всех 6 сторон (камень, земля).
    TopBottomSides = 1, // 3 текстуры: верх, низ и одинаковые 4 боковины (трава, книжная полка, кактус).
    AxisAligned = 2, // 2 текстуры: верх/низ — один тип, все боковины — второй тип (доски, песок).
    OrientedFront = 3, // 2 текстуры: одна сторона — лицо, остальные 5 — одинаковые (печка, раздатчик).
    Column = 4, // 3 текстуры: торцы (верх/низ), и боковины, которые зависят от направления (бревно).
    Individual = 5, // 6 текстур: каждая грань имеет уникальную текстуру.
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
