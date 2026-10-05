pub type Size = u64;

pub struct ConstPageAllocHelper<const BLOCKS_64_COUNT: usize> {
    pub bitset: [u64; BLOCKS_64_COUNT],
}

impl<const BLOCKS_64_COUNT: usize> Default for ConstPageAllocHelper<BLOCKS_64_COUNT> {
    fn default() -> Self {
        Self {
            bitset: [0; BLOCKS_64_COUNT],
        }
    }
}

#[derive(Clone, Copy)]
pub struct SectorAlloc<const SECTOR_SIZE: usize>(usize);

impl<const SECTOR_SIZE: usize> SectorAlloc<SECTOR_SIZE> {
    #[inline]
    pub fn get_bytes_offset(self) -> usize {
        self.0 * SECTOR_SIZE
    }

    pub fn get_sectors_offset(self) -> usize {
        self.0
    }
}

impl<const BLOCKS_64_COUNT: usize> ConstPageAllocHelper<BLOCKS_64_COUNT> {
    pub const PAGE_COUNT: usize = BLOCKS_64_COUNT * 64;

    pub const fn get_page_count(&self) -> usize {
        Self::PAGE_COUNT
    }

    pub fn free(&mut self, start_sector: usize, sectors_count: usize) {
        self.set_range(start_sector, sectors_count, false);
    }

    fn set_range(&mut self, start: usize, len: usize, value: bool) {
        let end = start + len;
        let start_u64_idx = start / 64;
        let start_bit_idx = start % 64;

        let end_u64_idx = (end - 1) / 64;
        let end_bit_idx = (end - 1) % 64;

        let start_mask = u64::MAX << start_bit_idx;
        let end_mask = u64::MAX >> (63 - end_bit_idx);

        let value_mask = ((value) as u64).wrapping_neg();

        if start_u64_idx == end_u64_idx {
            let combined_mask = start_mask & end_mask;
            self.bitset[start_u64_idx] =
                (self.bitset[start_u64_idx] & !combined_mask) | (value_mask & combined_mask);
        } else {
            self.bitset[start_u64_idx] =
                (self.bitset[start_u64_idx] & !start_mask) | (value_mask & start_mask);

            for i in (start_u64_idx + 1)..end_u64_idx {
                self.bitset[i] = value_mask;
            }

            self.bitset[end_u64_idx] =
                (self.bitset[end_u64_idx] & !end_mask) | (value_mask & end_mask);
        }
    }

    pub fn alloc<const SECTOR_SIZE: usize>(
        &mut self,
        needed_sectors: usize,
    ) -> SectorAlloc<SECTOR_SIZE> {
        let mut run_start = 0;
        let mut run_len = 0;

        // Считаем общее количество бит (секторов) в нашем битсете
        let total_sectors = self.bitset.len() * 64;

        let mut sector = 0;
        while sector < total_sectors {
            let u64_idx = sector / 64;
            let bit_idx = sector % 64;
            let word = self.bitset[u64_idx];

            // Проверяем, занят ли конкретный бит (сектор)
            // В Rust 0-й бит — это самый младший. Никаких reverse_bits не нужно!
            let is_occupied = (word & (1u64 << bit_idx)) != 0;

            if is_occupied {
                // Если сектор занят, текущая цепочка оборвалась. Сбрасываем длину
                run_len = 0;
                sector += 1;
            } else {
                // Если сектор свободен
                if run_len == 0 {
                    // Запоминаем начало новой цепочки секторов
                    run_start = sector;
                }
                run_len += 1;

                // Если нашли цепочку нужной длины — занимаем её и возвращаем адрес!
                if run_len >= needed_sectors {
                    self.set_range(run_start, needed_sectors, true);
                    return SectorAlloc(run_start);
                }

                // ОПТИМИЗАЦИЯ: Если мы попали на начало полностью пустого u64 слова,
                // и мы еще НЕ начали копить цепочку (run_len == 1, то есть этот сектор был первым),
                // мы можем мгновенно прошагнуть все 64 бита этого слова целиком!
                if bit_idx == 0 && word == 0 {
                    run_len = 64;
                    sector += 64;

                    // Проверяем, вдруг нам хватало ровно 64 или меньше секторов
                    if run_len >= needed_sectors {
                        self.set_range(run_start, needed_sectors, true);
                        return SectorAlloc(run_start);
                    }
                    continue;
                }

                sector += 1;
            }
        }

        // Если места не хватило (BLOCKS_64_COUNT * 64 + 1)
        SectorAlloc(total_sectors + 1)
    }
}
