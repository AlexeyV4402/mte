#[macro_export]
macro_rules! as_u8_slice {
    ($e:expr) => {
        unsafe {
            std::slice::from_raw_parts(
                ($e as *const _) as *const u8,
                std::mem::size_of_val($e)
            )
        }
    }
}

pub fn fill_fixed_vec(text: &str, size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    let bytes = text.as_bytes();
    let len = bytes.len().min(size);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

pub fn fill_fixed_array<const N: usize>(text: &str) -> [u8; N] {
    let mut array = [0u8; N];
    let bytes = text.as_bytes();

    let len = bytes.len().min(N);
    array[..len].copy_from_slice(&bytes[..len]);

    array
}
