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
