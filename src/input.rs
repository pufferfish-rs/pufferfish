//! Types relating to user input.

/// Symbolic names for virtual key codes.
#[repr(u8)]
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
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

    /// The '0' key above the alphabetic keys.
    Alpha0,
    /// The '1' key above the alphabetic keys.
    Alpha1,
    /// The '2' key above the alphabetic keys.
    Alpha2,
    /// The '3' key above the alphabetic keys.
    Alpha3,
    /// The '4' key above the alphabetic keys.
    Alpha4,
    /// The '5' key above the alphabetic keys.
    Alpha5,
    /// The '6' key above the alphabetic keys.
    Alpha6,
    /// The '7' key above the alphabetic keys.
    Alpha7,
    /// The '8' key above the alphabetic keys.
    Alpha8,
    /// The '9' key above the alphabetic keys.
    Alpha9,

    LeftControl,
    LeftShift,
    LeftAlt,
    RightControl,
    RightShift,
    RightAlt,

    Enter,
    Escape,
    Backspace,
    Tab,
    Space,

    PageUp,
    PageDown,
    End,
    Home,
    Insert,
    Delete,

    /// The '0' key on the numeric keypad.
    Num0,
    /// The '1' key on the numeric keypad.
    Num1,
    /// The '2' key on the numeric keypad.
    Num2,
    /// The '3' key on the numeric keypad.
    Num3,
    /// The '4' key on the numeric keypad.
    Num4,
    /// The '5' key on the numeric keypad.
    Num5,
    /// The '6' key on the numeric keypad.
    Num6,
    /// The '7' key on the numeric keypad.
    Num7,
    /// The '8' key on the numeric keypad.
    Num8,
    /// The '9' key on the numeric keypad.
    Num9,

    Left,
    Right,
    Up,
    Down,
}

/// An interface for querying user input. Accessible from [`App`](crate::App) by
/// default.
pub struct Input {
    pub(crate) keys_down: Vec<KeyCode>,
    pub(crate) keys_pressed: Vec<KeyCode>,
    pub(crate) keys_released: Vec<KeyCode>,
    pub(crate) chars_pressed: Vec<char>,
}

impl Input {
    pub(crate) fn new() -> Input {
        Input {
            keys_down: Vec::new(),
            keys_pressed: Vec::new(),
            keys_released: Vec::new(),
            chars_pressed: Vec::new(),
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

    /// Returns an iterator over all keys that were pressed since the last
    /// update.
    pub fn get_keys_pressed(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_pressed.iter().copied()
    }

    /// Returns an iterator over all keys that were released since the last
    /// update.
    pub fn get_keys_released(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys_released.iter().copied()
    }

    /// Drains and returns an iterator over all characters that have been
    /// pressed.
    pub fn get_chars_pressed(&mut self) -> impl Iterator<Item = char> + '_ {
        self.chars_pressed.drain(..)
    }
}
