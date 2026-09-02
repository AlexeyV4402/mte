use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use lib_core::math::vectors::vec3::core::Vector3;
use mte_macros::vpath;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::types::chunk::{Chunk, PackedChunk};
use crate::types::coordinates::core::{ChunkCoords, RegionCoords};
use crate::utils::sector_manager::SectorManager;

const FILE_FORMAT_LEN: usize = 16;
const FILE_FORMAT: &str = "AVC_MC_RGDT_FMT0";
const FILE_FORMAT_BYTES: &[u8; 16] = FILE_FORMAT.as_bytes().as_array().unwrap();

const _: () = assert!(
    FILE_FORMAT.len() == FILE_FORMAT_LEN,
    "Несовпадение длины формата файла"
);

#[repr(C)]
#[derive(Serialize, Deserialize)]
pub struct RegionHeader {
    pub file_format: [u8; FILE_FORMAT.len()],
    #[serde(with = "BigArray")]
    pub offsets: [u32; 32 * 32 * 32],
}

use serde_big_array::BigArray;

pub struct RegionData {
    pub offsets: [u32; 32 * 32 * 32],
    pub file_empty: bool,
}

impl Default for RegionData {
    fn default() -> Self {
        Self {
            offsets: [0u32; 32 * 32 * 32],
            file_empty: true,
        }
    }
}

pub struct SaveManager {
    regions: FxHashMap<RegionCoords, RegionData>,
    current_center: RegionCoords,
}

impl SaveManager {
    pub const SECTOR_SIZE: usize = 4096;
    pub const HEADER_SIZE_RAW: usize = 16 + (32 * 32 * 32) * 4;
    pub const OFFSETS_SIZE: usize = 32 * 32 * 32;
    pub const HEADER_SECTORS_USIZE: usize = Self::HEADER_SIZE_RAW.div_ceil(Self::SECTOR_SIZE);
    pub const HEADER_SECTORS_U32: u32 = Self::HEADER_SECTORS_USIZE as u32;
    pub const HEADER_SIZE_ALIGNED: usize = Self::HEADER_SECTORS_USIZE * Self::SECTOR_SIZE;

    pub fn new(current_center: RegionCoords) -> Self {
        let mut new = Self {
            regions: Default::default(),
            current_center,
        };

        let delta = Vector3::new(1, 1, 1);
        let old_min = current_center.0 - delta;
        let old_max = current_center.0 + delta;

        for x in old_min.x..=old_max.x {
            for y in old_min.y..=old_max.y {
                for z in old_min.z..=old_max.z {
                    let coords = RegionCoords::new(x, y, z);
                    let save_dir: PathBuf = vpath!("data://");
                    let region_coords = coords;
                    let file_name = format!(
                        "{}.{}.{}.rd",
                        region_coords.0.x, region_coords.0.y, region_coords.0.z
                    );
                    let file_path = save_dir.join(file_name);

                    let mut file = match File::open(&file_path) {
                        Ok(f) => f,
                        Err(_) => {
                            new.regions.insert(coords, RegionData::default());
                            continue;
                        }
                    };

                    match Self::read_region_header_from_file(&mut file) {
                        Ok(data) => {
                            new.regions.insert(
                                coords,
                                RegionData {
                                    offsets: data,
                                    file_empty: false,
                                },
                            );
                        }
                        Err(_) => {
                            new.regions.insert(coords, RegionData::default());
                        }
                    };
                }
            }
        }
        new
    }

    pub fn load_exist(&self, mut chunks: Vec<ChunkCoords>) -> Vec<(ChunkCoords, Option<Chunk>)> {
        let mut sorted: FxHashMap<RegionCoords, Vec<ChunkCoords>> = Default::default();

        chunks.drain(..).for_each(|chunk| {
            let vec = sorted.entry(chunk.get_region()).or_insert(Vec::default());
            vec.push(chunk);
        });

        let regions_batch: Vec<(RegionCoords, Vec<ChunkCoords>)> = sorted.into_iter().collect();

        let results: Vec<Vec<(ChunkCoords, Option<Chunk>)>> = regions_batch
            .into_par_iter()
            .map(|(region_coords, chunks_vec)| {
                let save_dir: PathBuf = vpath!("data://");
                let file_name = format!(
                    "{}.{}.{}.rd",
                    region_coords.0.x, region_coords.0.y, region_coords.0.z
                );
                let file_path = save_dir.join(file_name);

                let mut file = match File::open(&file_path) {
                    Ok(f) => f,
                    Err(_) => {
                        return chunks_vec.into_iter().map(|c| (c, None)).collect();
                    }
                };

                let cached_region = match self.get_region_data(region_coords) {
                    Some(region) => region,
                    None => return chunks_vec.into_iter().map(|c| (c, None)).collect(),
                };

                let mut local_results: Vec<(ChunkCoords, Option<Chunk>)> =
                    Vec::with_capacity(chunks_vec.len());

                for chunk_coords in chunks_vec {
                    let chunk_idx = usize::from(chunk_coords.in_region());
                    let chunk_header_data = cached_region.offsets[chunk_idx] as usize;

                    if chunk_header_data == 0 {
                        local_results.push((chunk_coords, None));
                        continue;
                    }

                    let sector_offset = chunk_header_data >> 8;

                    let absolute_offset = (sector_offset * Self::SECTOR_SIZE) as u64;
                    if file.seek(SeekFrom::Start(absolute_offset)).is_err() {
                        local_results.push((chunk_coords, None));
                        continue;
                    }

                    let mut len_bytes = [0u8; 4];
                    if file.read_exact(&mut len_bytes).is_err() {
                        local_results.push((chunk_coords, None));
                        continue;
                    }
                    let chunk_bytes_len = u32::from_be_bytes(len_bytes) as usize;

                    let mut chunk_buf = Vec::with_capacity(chunk_bytes_len);
                    unsafe {
                        chunk_buf.set_len(chunk_bytes_len);
                    }
                    if file.read_exact(&mut chunk_buf).is_err() {
                        local_results.push((chunk_coords, None));
                        continue;
                    }

                    let packed_chunk = match bincode::deserialize::<PackedChunk>(&chunk_buf) {
                        Ok(c) => c,
                        Err(_) => {
                            local_results.push((chunk_coords, None));
                            continue;
                        }
                    };

                    let chunk = Chunk {
                        data: packed_chunk.unpack(),
                        vram_slot_id: None,
                        is_changed: false,
                    };
                    local_results.push((chunk_coords, Some(chunk)));
                }
                local_results
            })
            .collect();

        results.into_iter().flatten().collect()
    }

    #[inline]
    pub fn get_region_data(&self, region_coords: RegionCoords) -> Option<&RegionData> {
        self.regions.get(&region_coords)
    }

    #[inline]
    pub fn get_region_data_mut(&mut self, region_coords: RegionCoords) -> Option<&mut RegionData> {
        self.regions.get_mut(&region_coords)
    }

    pub fn shift_center(&mut self, new_center: RegionCoords) {
        let delta = Vector3::new(1, 1, 1);
        let old_min = self.current_center.0 - delta;
        let old_max = self.current_center.0 + delta;
        let new_min = new_center.0 - delta;
        let new_max = new_center.0 + delta;

        let scan_min = old_min.min_cw(new_min);
        let scan_max = old_max.max_cw(new_max);

        for x in scan_min.x..=scan_max.x {
            for y in scan_min.y..=scan_max.y {
                for z in scan_min.z..=scan_max.z {
                    let coords = RegionCoords::new(x, y, z);

                    let in_old = x >= old_min.x
                        && x <= old_max.x
                        && y >= old_min.y
                        && y <= old_max.y
                        && z >= old_min.z
                        && z <= old_max.z;

                    let in_new = x >= new_min.x
                        && x <= new_max.x
                        && y >= new_min.y
                        && y <= new_max.y
                        && z >= new_min.z
                        && z <= new_max.z;

                    if !in_old && in_new {
                        let save_dir: PathBuf = vpath!("data://");
                        let region_coords = coords;
                        let file_name = format!(
                            "{}.{}.{}.rd",
                            region_coords.0.x, region_coords.0.y, region_coords.0.z
                        );
                        let file_path = save_dir.join(file_name);

                        let mut file = match File::open(&file_path) {
                            Ok(f) => f,
                            Err(_) => {
                                self.regions.insert(coords, RegionData::default());
                                continue;
                            }
                        };

                        match Self::read_region_header_from_file(&mut file) {
                            Ok(data) => {
                                self.regions.insert(
                                    coords,
                                    RegionData {
                                        offsets: data,
                                        file_empty: false,
                                    },
                                );
                            }
                            Err(_) => {
                                self.regions.insert(coords, RegionData::default());
                            }
                        };
                    }

                    if in_old && !in_new {
                        self.regions.remove(&coords);
                    }
                }
            }
        }
        self.current_center = new_center;
    }

    pub fn save_chunks(&mut self, mut chunks: Vec<(ChunkCoords, Chunk)>) {
        let mut sorted: FxHashMap<RegionCoords, Vec<(ChunkCoords, Chunk)>> = Default::default();

        chunks.drain(..).for_each(|data| {
            let vec = sorted.entry(data.0.get_region()).or_insert(Vec::default());
            vec.push(data);
        });

        let regions_batch: Vec<(RegionCoords, Vec<(ChunkCoords, Chunk)>)> =
            sorted.into_iter().collect();

        let mut par_tasks = Vec::with_capacity(regions_batch.len());

        for (region_coords, chunks_vec) in regions_batch {
            let region_data = match self.get_region_data_mut(region_coords) {
                Some(region) => unsafe { &mut *(region as *mut RegionData) },
                None => continue,
            };

            par_tasks.push((region_coords, chunks_vec, region_data));
        }

        par_tasks
            .into_par_iter()
            .for_each(|(region_coords, chunks_vec, cached_region)| {
                let save_dir: PathBuf = vpath!("data://");
                let file_name = format!(
                    "{}.{}.{}.rd",
                    region_coords.0.x, region_coords.0.y, region_coords.0.z
                );
                let file_path = save_dir.join(file_name);

                let mut file = if let Ok(f) = OpenOptions::new()
                    .create(true)
                    .read(true)
                    .write(true)
                    .open(&file_path)
                {
                    f
                } else {
                    return;
                };

                let local_offsets = &mut cached_region.offsets;

                let mut sector_manager =
                    SectorManager::for_header(local_offsets, Self::HEADER_SIZE_RAW as u64);

                if !cached_region.file_empty {
                    for (chunk_coords, chunk_data) in chunks_vec {
                        let chunk_idx = usize::from(chunk_coords.in_region());
                        let chunk_header_data = local_offsets[chunk_idx] as usize;

                        let sector_offset: usize;

                        let packed_chunk_bytes =
                            match bincode::serialize(&PackedChunk::pack(&chunk_data.data)) {
                                Ok(p) => p,
                                Err(_) => continue,
                            };

                        let need_sectors =
                            (4 + packed_chunk_bytes.len()).div_ceil(Self::SECTOR_SIZE);

                        if chunk_header_data == 0 {
                            sector_offset = sector_manager.alloc(need_sectors);
                        } else {
                            let current_sectors = chunk_header_data & 0xFF;

                            if need_sectors > current_sectors {
                                sector_manager.free(chunk_header_data >> 8, current_sectors);
                                sector_offset = sector_manager.alloc(need_sectors);
                            } else {
                                sector_offset = chunk_header_data >> 8;
                            }
                        }

                        let len_bytes = (packed_chunk_bytes.len() as u32).to_be_bytes();

                        let mut write_buf: Box<[u8]> = unsafe {
                            let layout = std::alloc::Layout::from_size_align(
                                need_sectors * Self::SECTOR_SIZE,
                                1,
                            )
                            .unwrap();
                            let ptr = std::alloc::alloc(layout);
                            Box::from_raw(std::slice::from_raw_parts_mut(
                                ptr,
                                need_sectors * Self::SECTOR_SIZE,
                            ))
                        };

                        unsafe {
                            let buf_ptr = write_buf.as_mut_ptr();
                            buf_ptr.copy_from_nonoverlapping(len_bytes.as_ptr(), 4);
                            buf_ptr.add(4).copy_from_nonoverlapping(
                                packed_chunk_bytes.as_ptr(),
                                packed_chunk_bytes.len(),
                            );
                        }

                        local_offsets[chunk_idx] = (sector_offset << 8 | need_sectors) as u32;

                        if file
                            .seek(SeekFrom::Start((sector_offset * Self::SECTOR_SIZE) as u64))
                            .is_ok()
                        {
                            let _ = file.write_all(&write_buf);
                        }
                    }

                    let mut header_offsets = [0u32; Self::OFFSETS_SIZE];
                    header_offsets[..Self::OFFSETS_SIZE]
                        .copy_from_slice(&local_offsets[..Self::OFFSETS_SIZE]);

                    let header = RegionHeader {
                        file_format: *FILE_FORMAT_BYTES,
                        offsets: header_offsets,
                    };

                    if file.seek(SeekFrom::Start(0)).is_ok() {
                        if let Ok(b) = bincode::serialize::<RegionHeader>(&header) {
                            let _ = file.write_all(&b);
                        }
                    }
                } else {
                    // file.is_empty()
                    let mut chunks_write_buf: Vec<u8> = Vec::new();

                    for (chunk_coords, chunk_data) in chunks_vec {
                        let chunk_idx = usize::from(chunk_coords.in_region());

                        if chunk_coords.0 == Vector3::new(0, 2, 0) {
                            println!("empty: Сохранение чанка: {};", chunk_coords.0);
                        };

                        let packed_chunk_bytes =
                            match bincode::serialize(&PackedChunk::pack(&chunk_data.data)) {
                                Ok(p) => p,
                                Err(_) => continue,
                            };

                        let need_sectors =
                            (4 + packed_chunk_bytes.len()).div_ceil(Self::SECTOR_SIZE);

                        let sector_offset = sector_manager.alloc(need_sectors);

                        local_offsets[chunk_idx] = (sector_offset << 8 | need_sectors) as u32;

                        let len_bytes = (packed_chunk_bytes.len() as u32).to_be_bytes();
                        chunks_write_buf.extend_from_slice(&len_bytes);
                        chunks_write_buf.extend_from_slice(&packed_chunk_bytes);

                        let bytes_written = 4 + packed_chunk_bytes.len();
                        let total_allocated_bytes = need_sectors * Self::SECTOR_SIZE;
                        let padding_needed = total_allocated_bytes - bytes_written;
                        if padding_needed > 0 {
                            const ZEROES: [u8; 4096] = [0u8; 4096];
                            chunks_write_buf.extend_from_slice(&ZEROES[..padding_needed]);
                        }
                    }

                    let mut header_offsets = [0u32; Self::OFFSETS_SIZE];
                    header_offsets[..Self::OFFSETS_SIZE]
                        .copy_from_slice(&local_offsets[..Self::OFFSETS_SIZE]);

                    let header = RegionHeader {
                        file_format: *FILE_FORMAT_BYTES,
                        offsets: header_offsets,
                    };

                    let header_bytes = bincode::serialize::<RegionHeader>(&header).unwrap();

                    let total_file_size = Self::HEADER_SIZE_ALIGNED + chunks_write_buf.len();
                    let mut write_buf: Box<[u8]> = unsafe {
                        let layout =
                            std::alloc::Layout::from_size_align(total_file_size, 1).unwrap();
                        let ptr = std::alloc::alloc(layout);
                        Box::from_raw(std::slice::from_raw_parts_mut(ptr, total_file_size))
                    };

                    unsafe {
                        let buf_ptr = write_buf.as_mut_ptr();
                        buf_ptr.copy_from_nonoverlapping(header_bytes.as_ptr(), header_bytes.len());
                        buf_ptr
                            .add(Self::HEADER_SIZE_ALIGNED)
                            .copy_from_nonoverlapping(
                                chunks_write_buf.as_ptr(),
                                chunks_write_buf.len(),
                            );
                    }

                    if file.seek(SeekFrom::Start(0)).is_ok() {
                        let _ = file.write_all(&write_buf);
                    }

                    cached_region.file_empty = false;
                }
            })
    }

    pub fn read_region_header_from_file(file: &mut File) -> Result<[u32; 32 * 32 * 32], String> {
        let mut buf = [0u8; Self::HEADER_SIZE_RAW];
        file.read_exact(&mut buf)
            .map_err(|e| format!("ошибка чтения заголовка: {}", e))?;

        let header = bincode::deserialize::<RegionHeader>(&buf)
            .map_err(|e| format!("ошибка десериализации заголовка: {}", e))?;

        if header.file_format != *FILE_FORMAT_BYTES {
            return Err("формат файла не соответствует требуемому.".to_string());
        }

        Ok(header.offsets)
    }
}
