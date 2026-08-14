use core::fmt;
use std::marker::PhantomData;

pub type LocalCoordsType = u32;
pub type InternalCoordsType = u32;
pub type GlobalCoordsType = i64;
pub type ChunkCoordsType = i32;

pub type LocalCoords = Coords<Local, LocalCoordsType>;
pub type InternalCoords = Coords<Internal, InternalCoordsType>;
pub type GlobalCoords = Coords<Global, GlobalCoordsType>;
pub type ChunkCoords = Coords<Chunk, ChunkCoordsType>;

#[derive(Clone, Copy)]
pub struct Local;

#[derive(Clone, Copy)]
pub struct Global;

#[derive(Clone, Eq, Hash, PartialEq, Copy)]
pub struct Chunk;

#[derive(Clone, Copy)]
pub struct Internal;

#[repr(C)]
#[derive(Clone, Copy, Eq, Hash, PartialEq, Default)]
pub struct Coords<S, N> {
    pub(super) x: N,
    pub(super) y: N,
    pub(super) z: N,
    pub(super) _marker: PhantomData<S>,
}

impl<S, N> From<Coords<S, N>> for (N, N, N) {
    fn from(coords: Coords<S, N>) -> Self {
        (coords.x, coords.y, coords.z)
    }
}

impl<S, N> From<(N, N, N)> for Coords<S, N> {
    fn from(coords: (N, N, N)) -> Self {
        Self {
            x: coords.0,
            y: coords.1,
            z: coords.2,
            _marker: PhantomData,
        }
    }
}

impl<S, N: fmt::Debug> fmt::Debug for Coords<S, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Coords") // Имя структуры в выводе
            .field("x", &self.x) // Выводим поле x
            .field("y", &self.y) // Выводим поле y
            .field("z", &self.z) // Выводим поле z
            .finish() // Завершаем сборку
    }
}
