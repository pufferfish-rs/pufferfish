use sdl2::keyboard::Keycode as SDLKeyCode;

#[repr(u8)]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum KeyCode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    Alpha0,
    Alpha1,
    Alpha2,
    Alpha3,
    Alpha4,
    Alpha5,
    Alpha6,
    Alpha7,
    Alpha8,
    Alpha9,

    LeftControl,
    LeftShift,
    LeftAlt,
    RightControl,
    RightShift,
    RightAlt,
}

pub(crate) fn keycode_from_sdl(keycode: Option<SDLKeyCode>) -> Option<KeyCode> {
    keycode.and_then(|keycode| match keycode {
        SDLKeyCode::A => Some(KeyCode::A),
        SDLKeyCode::B => Some(KeyCode::B),
        SDLKeyCode::C => Some(KeyCode::C),
        SDLKeyCode::D => Some(KeyCode::D),
        SDLKeyCode::E => Some(KeyCode::E),
        SDLKeyCode::F => Some(KeyCode::F),
        SDLKeyCode::G => Some(KeyCode::G),
        SDLKeyCode::H => Some(KeyCode::H),
        SDLKeyCode::I => Some(KeyCode::I),
        SDLKeyCode::J => Some(KeyCode::J),
        SDLKeyCode::K => Some(KeyCode::K),
        SDLKeyCode::L => Some(KeyCode::L),
        SDLKeyCode::M => Some(KeyCode::M),
        SDLKeyCode::N => Some(KeyCode::N),
        SDLKeyCode::O => Some(KeyCode::O),
        SDLKeyCode::P => Some(KeyCode::P),
        SDLKeyCode::Q => Some(KeyCode::Q),
        SDLKeyCode::R => Some(KeyCode::R),
        SDLKeyCode::S => Some(KeyCode::S),
        SDLKeyCode::T => Some(KeyCode::T),
        SDLKeyCode::U => Some(KeyCode::U),
        SDLKeyCode::V => Some(KeyCode::V),
        SDLKeyCode::W => Some(KeyCode::W),
        SDLKeyCode::X => Some(KeyCode::X),
        SDLKeyCode::Y => Some(KeyCode::Y),
        SDLKeyCode::Z => Some(KeyCode::Z),

        SDLKeyCode::Num0 => Some(KeyCode::Alpha0),
        SDLKeyCode::Num1 => Some(KeyCode::Alpha1),
        SDLKeyCode::Num2 => Some(KeyCode::Alpha2),
        SDLKeyCode::Num3 => Some(KeyCode::Alpha3),
        SDLKeyCode::Num4 => Some(KeyCode::Alpha4),
        SDLKeyCode::Num5 => Some(KeyCode::Alpha5),
        SDLKeyCode::Num6 => Some(KeyCode::Alpha6),
        SDLKeyCode::Num7 => Some(KeyCode::Alpha7),
        SDLKeyCode::Num8 => Some(KeyCode::Alpha8),
        SDLKeyCode::Num9 => Some(KeyCode::Alpha9),

        SDLKeyCode::LCtrl => Some(KeyCode::LeftControl),
        SDLKeyCode::LShift => Some(KeyCode::LeftShift),
        SDLKeyCode::LAlt => Some(KeyCode::LeftAlt),
        SDLKeyCode::RCtrl => Some(KeyCode::RightControl),
        SDLKeyCode::RShift => Some(KeyCode::RightShift),
        SDLKeyCode::RAlt => Some(KeyCode::RightAlt),

        _ => None,
    })
}

pub struct Input {
    pub(crate) keys_down: Vec<KeyCode>,
    pub(crate) keys_pressed: Vec<KeyCode>,
    pub(crate) keys_released: Vec<KeyCode>,
}

impl Input {
    pub(crate) fn new() -> Input {
        Input {
            keys_down: Vec::new(),
            keys_pressed: Vec::new(),
            keys_released: Vec::new(),
        }
    }

    pub(crate) fn update(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
    }

    /// Returns true if the specified key is currently down.
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Returns true if the specified key was pressed since the last update.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Returns true if the specified key was released since the last update.
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Returns an iterator over all keys that are currently down.
    pub fn get_keys_down(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_down.iter().copied()
    }

    /// Returns an iterator over all keys that were pressed since the last update.
    pub fn get_keys_pressed(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_pressed.iter().copied()
    }

    /// Returns an iterator over all keys that were released since the last update.
    pub fn get_keys_released(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_released.iter().copied()
    }
}
