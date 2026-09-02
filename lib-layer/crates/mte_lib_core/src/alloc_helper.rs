pub struct ConstPageAllocHelper<const BLOCKS_64_COUNT: usize> {
    pub bitset: [u64; BLOCKS_64_COUNT],
}

impl<const BLOCKS_64_COUNT: usize> ConstPageAllocHelper<BLOCKS_64_COUNT> {
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

    pub fn alloc(&mut self, needed_sectors: usize) -> usize {
        let mut run_start = 0;
        let mut run_len = 0;

        for u64_idx in 0..self.bitset.len() {
            let word = self.bitset[u64_idx];
            let base_sector = u64_idx * 64;

            if word == u64::MAX {
                run_len = 0;
                continue;
            }

            if word == 0 {
                if run_len == 0 {
                    run_start = base_sector;
                }
                run_len += 64;

                if run_len >= needed_sectors {
                    self.set_range(run_start, needed_sectors, true);
                    return run_start;
                }
                continue;
            }

            let word_reversed = word.reverse_bits();
            let mut bit_idx = 0;
            while bit_idx < 64 {
                let is_occupied = (word_reversed & (1u64 << bit_idx)) != 0;

                let occupied = (word_reversed >> bit_idx).trailing_ones() as usize;

                if is_occupied {
                    run_len = 0;
                    bit_idx += occupied.min(64 - bit_idx);
                } else {
                    if run_len == 0 {
                        run_start = base_sector + bit_idx;
                    }
                    run_len += 1;

                    if run_len >= needed_sectors {
                        self.set_range(run_start, needed_sectors, true);
                        return run_start;
                    }
                    bit_idx += 1;
                }
            }
        }

        BLOCKS_64_COUNT * 64 + 1
    }
}
