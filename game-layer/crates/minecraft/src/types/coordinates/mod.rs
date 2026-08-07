use std::fmt;

pub mod chunk;
pub mod global;
pub mod local;

pub type LocalCoordsType = u32;
pub type GlobalCoordsType = i64;
pub type ChunkCoordsType = i32;

pub type LocalCoords = Coords<LocalCoordsType>;
pub type GlobalCoords = Coords<GlobalCoordsType>;
pub type ChunkCoords = Coords<ChunkCoordsType>;

#[repr(C)]
#[derive(Clone, Copy, Eq, Hash, PartialEq, Default)]
pub struct Coords<T> {
    x: T,
    y: T,
    z: T,
}

impl<T> From<Coords<T>> for (T, T, T) {
    fn from(coords: Coords<T>) -> Self {
        (coords.x, coords.y, coords.z)
    }
}

impl<T> From<(T, T, T)> for Coords<T> {
    fn from(coords: (T, T, T)) -> Self {
        Self {
            x: coords.0,
            y: coords.1,
            z: coords.2,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Coords<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Coords") // Имя структуры в выводе
            .field("x", &self.x) // Выводим поле x
            .field("y", &self.y) // Выводим поле y
            .field("z", &self.z) // Выводим поле z
            .finish() // Завершаем сборку
    }
}
