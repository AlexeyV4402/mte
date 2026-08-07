use glam::Vec2;
use winit::event::{DeviceEvent, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

const MOUSE_OFFSET: u32 = 256;
const MOUSE_PIXEL_COEFFICIENT: f32 = 0.1;

#[derive(Default)]
pub struct InputState {
    keys_down: [u64; 5],
    keys_down_prev: [u64; 5],
    pub mouse_scroll_delta: f32,
    pub mouse_pos: Vec2,
    pub raw_mouse_delta: Vec2,
}

impl InputState {
    #[inline(always)]
    fn get_idx_and_bit(bit_id: u32) -> (usize, u64) {
        let idx = (bit_id >> 6) as usize; // Деление на 64 (сдвиг на 6 бит)
        let bit = 1 << (bit_id & 63); // Остаток от деления на 64 в качестве сдвига бита
        (idx, bit)
    }

    #[inline(always)]
    pub fn set_bit(&mut self, bit_id: u32, value: bool) {
        let (idx, bit) = Self::get_idx_and_bit(bit_id);
        if value {
            self.keys_down[idx] |= bit;
        } else {
            self.keys_down[idx] &= !bit;
        }
    }

    #[inline(always)]
    pub fn get_bit(&self, bit_id: u32) -> bool {
        let (idx, bit) = Self::get_idx_and_bit(bit_id);
        (self.keys_down[idx] & bit) != 0
    }

    #[inline(always)]
    pub fn get_prev_bit(&self, bit_id: u32) -> bool {
        // Поменял &mut self на &self
        let (idx, bit) = Self::get_idx_and_bit(bit_id);
        (self.keys_down_prev[idx] & bit) != 0
    }

    pub fn is_down(&self, key_code: KeyCode) -> bool {
        self.get_bit(key_code as u32)
    }

    pub fn is_just_pressed(&self, key_code: KeyCode) -> bool {
        let id = key_code as u32;
        self.get_bit(id) && !self.get_prev_bit(id)
    }

    pub fn set_mouse_bit(&mut self, button_id: u32, value: bool) {
        self.set_bit(MOUSE_OFFSET + button_id, value);
    }

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        let id = MOUSE_OFFSET + Self::mouse_button_to_u32(button);
        self.get_bit(id)
    }

    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        let id = MOUSE_OFFSET + Self::mouse_button_to_u32(button);
        self.get_bit(id) && !self.get_prev_bit(id)
    }

    fn mouse_button_to_u32(button: MouseButton) -> u32 {
        match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            MouseButton::Back => 3,
            MouseButton::Forward => 4,
            MouseButton::Other(id) => 5 + id as u32,
        }
    }

    pub fn update(&mut self) {
        self.keys_down_prev = self.keys_down;
        self.raw_mouse_delta = Vec2::ZERO;
        self.mouse_scroll_delta = 0.0;
    }

    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                self.set_bit(*code as u32, state.is_pressed());
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let idx = Self::mouse_button_to_u32(button.clone());
                self.set_mouse_bit(idx, state.is_pressed());
            }
            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = glam::vec2(position.x as f32, position.y as f32);
                self.mouse_pos = new_pos;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    // Для колеса
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                    // Для тачпада
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        pos.y as f32 * MOUSE_PIXEL_COEFFICIENT
                    }
                };
                self.mouse_scroll_delta += scroll_amount;
            }
            _ => {}
        }
    }

    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.raw_mouse_delta += glam::Vec2::new(delta.0 as f32, delta.1 as f32);
        }
    }
}

// Нужно переделать управление с учётом возможности ввода с нескольких устройств (например, для раздельного экрана).
// Также добавить геймпады.
