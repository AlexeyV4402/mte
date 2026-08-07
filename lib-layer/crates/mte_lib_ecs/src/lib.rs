// pub fn add(left: u64, right: u64) -> u64 {
//     left + right
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }

pub use std::alloc::{Layout, alloc, dealloc, handle_alloc_error};
pub use std::ptr::{copy, copy_nonoverlapping};

pub struct TableLayout {
    pub offsets: Vec<usize>, // Смещения для каждой колонки
    pub full_layout: Layout, // Общий лейаут для аллокации памяти
}

impl TableLayout {
    pub fn new(component_types: &[Layout], capacity: usize) -> Self {
        let mut offsets = Vec::with_capacity(component_types.len());
        let mut current_offset = 0;
        let mut max_align = 1;

        for layout in component_types {
            // 1. Выравниваем текущий адрес под нужды этого компонента
            // Например, если current_offset = 1, а align = 4, то станет 4
            let align = layout.align();
            current_offset = (current_offset + align - 1) & !(align - 1);

            // 2. Запоминаем начало этой колонки
            offsets.push(current_offset);

            // 3. Добавляем размер всей колонки (размер типа * количество сущностей)
            current_offset += layout.size() * capacity;

            // 4. Общий Align всей таблицы должен быть равен МАКСИМАЛЬНОМУ из всех компонентов
            max_align = max_align.max(align);
        }

        // Создаем финальный лейаут для всего блока памяти
        let full_layout =
            Layout::from_size_align(current_offset, max_align).expect("Ошибка при создании Layout");

        Self {
            offsets,
            full_layout,
        }
    }
}

pub trait ArchetypeOperations {
    fn new() -> Self;
    fn grow(&mut self);
    unsafe fn get_component_ptr<T: 'static>(&self, col_idx: usize, row_idx: usize) -> *mut T;
    unsafe fn drop_all_at_row(&self, row: usize);
    unsafe fn remove(&mut self, row: usize);
    unsafe fn delete(&mut self, row: usize);
}

#[macro_export]
macro_rules! get_layouts {
    ($($comp:ty),*) => {
        [ $( std::alloc::Layout::new::<$comp>() ),* ]
    }
}

#[macro_export]
macro_rules! count_types {
    ($($t:ty),*) => {
        <[()]>::len(&[ $(  $crate::count_types!(@subst $t)  ),* ])
    };
    (@subst $t:ty) => { () };
}

#[macro_export]
macro_rules! define_archetype_storage {
($name:ident, $($name_field:ident: $type_field:ty),*) => {

pub struct $name {
    ptr: *mut u8,
    offsets: [usize; $crate::count_types!($($type_field),*)],
    capacity: usize,
    len: usize,
    full_layout: $crate::Layout,
    component_layouts: [$crate::Layout; $crate::count_types!($($type_field),*)]
}

impl $name {
    pub unsafe fn push(&mut self, $($name_field: $type_field),*) {
        if self.len == self.capacity {
            self.grow();
        }
        let mut col_idx = 0;
        $(
            let ptr = self.get_component_ptr::<$type_field>(col_idx, self.len);
            std::ptr::write(ptr, $name_field);
            col_idx += 1;
        )*
        self.len += 1;
    }
}


impl $crate::ArchetypeOperations for $name {

    fn new() -> Self {
        let layouts = $crate::get_layouts!($($type_field),*);
        let layout_table = $crate::TableLayout::new(&layouts, 4);

        Self {
            ptr: std::ptr::null_mut(),
            offsets: [0usize; $crate::count_types!($($type_field),*)],
            capacity: 0,
            len: 0,
            full_layout: layout_table.full_layout,
            component_layouts: layouts,
        }
    }

    unsafe fn get_component_ptr<T: 'static>(&self, col_idx: usize, row_idx: usize) -> *mut T {
        let offset = self.offsets[col_idx];
        self.ptr.add(offset + row_idx * std::mem::size_of::<T>()) as *mut T
    }

    unsafe fn drop_all_at_row(&self, row: usize) {
        let mut _col = 0;
        $(
            if std::mem::needs_drop::<$type_field>() {
                let ptr = self.get_component_ptr::<$type_field>(_col, row);
                std::ptr::drop_in_place(ptr);
            }
            _col += 1;
        )*
    }

    #[cold]
    #[inline(never)]
    fn grow(&mut self) {
        self.capacity = (self.capacity << 1).max(8);;

        let table_layout = $crate::TableLayout::new(&self.component_layouts, self.capacity);

        unsafe {
            let ptr_new = $crate::alloc(table_layout.full_layout);
            if ptr_new.is_null() {
                $crate::handle_alloc_error(table_layout.full_layout);
            }

            for (i, _) in self.offsets.iter().enumerate() {
                $crate::copy_nonoverlapping(self.ptr.add(self.offsets[i]), ptr_new.add(table_layout.offsets[i]), self.len * self.component_layouts[i].size());
            }

            $crate::dealloc(self.ptr, self.full_layout);

            self.offsets = {
                let mut arr = [0usize; $crate::count_types!($($type_field),*)];
                arr.copy_from_slice(&table_layout.offsets);
                arr
            };
            self.ptr = ptr_new;
        }
    }

    unsafe fn remove(&mut self, row_to_remove: usize) {
        let last_row_idx = self.len - 1;

        for (col_idx, &offset) in self.offsets.iter().enumerate() {
            let size = self.component_layouts[col_idx].size();
            let src = self.ptr.add(offset + last_row_idx * size);
            let dst = self.ptr.add(offset + row_to_remove * size);

            std::ptr::copy(src, dst, size);
        }
        self.len -= 1;
    }

    unsafe fn delete(&mut self, row_to_remove: usize) {
        let last_row_idx = self.len - 1;

        self.drop_all_at_row(row_to_remove);

        for (col_idx, &offset) in self.offsets.iter().enumerate() {
            let size = self.component_layouts[col_idx].size();
            let src = self.ptr.add(offset + last_row_idx * size);
            let dst = self.ptr.add(offset + row_to_remove * size);

            std::ptr::copy(src, dst, size);
        }

        //self.update_index_callback(last_row_idx, row_to_remove);

        self.len -= 1;
    }
}
};}
