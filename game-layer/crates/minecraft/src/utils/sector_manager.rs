pub struct SectorManager {
    bitset: Vec<u64>,
    total_sectors: usize,
}

impl SectorManager {
    pub fn for_header(offsets: &[u32], file_size: u64) -> Self {
        let mut total_sectors = (file_size as usize).div_ceil(4096);

        let u64_count = total_sectors.div_ceil(64);
        let mut bitset = vec![0u64; u64_count.max(1)];

        Self::preset_range(&mut bitset, 0, 33, true);

        for &packed_offset in offsets.iter() {
            if packed_offset != 0 {
                let sector_idx = (packed_offset >> 8) as usize;
                let sectors_count = (packed_offset & 0xFF) as usize;

                for s in sector_idx..(sector_idx + sectors_count) {
                    let u64_idx = s / 64;
                    let bit_idx = s % 64;
                    if u64_idx >= bitset.len() {
                        bitset.resize(u64_idx + 1, 0);
                    }
                    bitset[u64_idx] |= 1 << bit_idx;
                }
            }
        }

        total_sectors = bitset.len() * 64;

        Self {
            bitset,
            total_sectors,
        }
    }

    pub fn display_word(&self, idx: usize) -> String {
        format!("{:0<64b}", self.bitset[idx])
    }

    pub fn free(&mut self, start_sector: usize, sectors_count: usize) {
        self.set_range(start_sector, sectors_count, false);
    }

    #[inline(always)]
    const fn preset_range(bitset: &mut [u64], start: usize, len: usize, value: bool) {
        let end = start + len;
        let start_u64_idx = start / 64;
        let start_bit_idx = start % 64;

        let end_u64_idx = (end - 1) / 64;
        let end_bit_idx = (end - 1) % 64;

        let start_mask = u64::MAX << start_bit_idx;
        let end_mask = u64::MAX >> (63 - end_bit_idx);

        let value_mask = ((value) as u64).wrapping_neg();

        assert!(start_u64_idx == end_u64_idx, "start_u64_idx != end_u64_idx");
        let combined_mask = start_mask & end_mask;
        bitset[start_u64_idx] =
            (bitset[start_u64_idx] & !combined_mask) | (value_mask & combined_mask);
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

    pub fn alloc(&mut self, needed_sectors: usize) -> usize {
        let mut run_start = 0;
        let mut run_len = 0;
        let total_sectors = self.bitset.len() * 64;
        let mut sector = 0;

        while sector < total_sectors {
            let u64_idx = sector / 64;
            let bit_idx = sector % 64;
            let word = self.bitset[u64_idx];

            // ОПТИМИЗАЦИЯ: Если мы стоим на начале полностью пустого u64 слова,
            // мы можем мгновенно прибавить все 64 бита к текущей цепочке!
            if bit_idx == 0 && word == 0 {
                if run_len == 0 {
                    run_start = sector;
                }
                run_len += 64; // ИСПРАВЛЕНО: Плюсуем, а не затираем!

                if run_len >= needed_sectors {
                    self.set_range(run_start, needed_sectors, true);
                    return run_start;
                }

                sector += 64; // Прыгаем на следующее слово
                continue;
            }

            // Стандартный побитовый обсчет (если слово заполнено частично)
            let is_occupied = (word & (1u64 << bit_idx)) != 0;

            if is_occupied {
                run_len = 0; // Цепочка прервалась
                sector += 1;
            } else {
                if run_len == 0 {
                    run_start = sector;
                }
                run_len += 1;

                if run_len >= needed_sectors {
                    self.set_range(run_start, needed_sectors, true);
                    return run_start;
                }

                sector += 1;
            }
        }

        let start_at_end = self.total_sectors;
        self.total_sectors += needed_sectors;

        let required_u64_len = self.total_sectors.div_ceil(64);
        if required_u64_len > self.bitset.len() {
            self.bitset.resize(required_u64_len, 0);
        }

        self.set_range(start_at_end, needed_sectors, true);
        start_at_end
    }

    pub fn alloc_debug(&mut self, needed_sectors: usize) -> usize {
        println!("alloc");
        let mut run_start = 0;
        let mut run_len = 0;
        let total_sectors = self.bitset.len() * 64;
        let mut sector = 0;

        while sector < total_sectors {
            let u64_idx = sector / 64;
            let bit_idx = sector % 64;
            let word = self.bitset[u64_idx];
            println!("u64_idx: {}, word: {}", u64_idx, self.display_word(u64_idx));

            // ОПТИМИЗАЦИЯ: Если мы стоим на начале полностью пустого u64 слова,
            // мы можем мгновенно прибавить все 64 бита к текущей цепочке!
            if bit_idx == 0 && word == 0 {
                if run_len == 0 {
                    run_start = sector;
                }
                run_len += 64; // ИСПРАВЛЕНО: Плюсуем, а не затираем!

                if run_len >= needed_sectors {
                    self.set_range(run_start, needed_sectors, true);
                    println!("return type 1");
                    return run_start;
                }

                sector += 64; // Прыгаем на следующее слово
                continue;
            }

            // Стандартный побитовый обсчет (если слово заполнено частично)
            let is_occupied = (word & (1u64 << bit_idx)) != 0;

            if is_occupied {
                run_len = 0; // Цепочка прервалась
                sector += 1;
            } else {
                if run_len == 0 {
                    run_start = sector;
                }
                run_len += 1;

                if run_len >= needed_sectors {
                    self.set_range(run_start, needed_sectors, true);
                    println!("return type 2");
                    return run_start;
                }

                sector += 1;
            }
        }

        let start_at_end = self.total_sectors;
        self.total_sectors += needed_sectors;

        let required_u64_len = self.total_sectors.div_ceil(64);
        if required_u64_len > self.bitset.len() {
            self.bitset.resize(required_u64_len, 0);
        }

        self.set_range(start_at_end, needed_sectors, true);
        println!("return type 3");
        start_at_end
    }
}
