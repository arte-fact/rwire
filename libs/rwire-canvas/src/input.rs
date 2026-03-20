//! Input state decoded from client binary messages.

/// Keyboard bitmask constants.
pub const KEY_UP: u8 = 0x01;
pub const KEY_LEFT: u8 = 0x02;
pub const KEY_DOWN: u8 = 0x04;
pub const KEY_RIGHT: u8 = 0x08;
pub const KEY_ATTACK: u8 = 0x10;
pub const KEY_ORDER_HOLD: u8 = 0x20;
pub const KEY_ORDER_GO: u8 = 0x40;
pub const KEY_ORDER_RETREAT: u8 = 0x80;

/// Second key byte for additional keys.
pub const KEY2_ORDER_FOLLOW: u8 = 0x01;
pub const KEY2_ZOOM_IN: u8 = 0x02;
pub const KEY2_ZOOM_OUT: u8 = 0x04;

/// Decoded input state from the client.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// Keyboard bitmask (see KEY_* constants).
    pub keys: u8,
    /// Second key byte for overflow keys.
    pub keys2: u8,
    /// Touch/mouse X position (canvas-relative pixels).
    pub touch_x: i16,
    /// Touch/mouse Y position (canvas-relative pixels).
    pub touch_y: i16,
    /// Whether the touch/mouse is active.
    pub touching: bool,
}

impl InputState {
    pub fn key(&self, mask: u8) -> bool {
        self.keys & mask != 0
    }

    pub fn key2(&self, mask: u8) -> bool {
        self.keys2 & mask != 0
    }

    /// Get movement direction as (dx, dy) normalized to -1/0/1.
    pub fn move_dir(&self) -> (i8, i8) {
        let mut dx = 0i8;
        let mut dy = 0i8;
        if self.key(KEY_UP) { dy -= 1; }
        if self.key(KEY_DOWN) { dy += 1; }
        if self.key(KEY_LEFT) { dx -= 1; }
        if self.key(KEY_RIGHT) { dx += 1; }
        (dx, dy)
    }

    pub fn attacking(&self) -> bool {
        self.key(KEY_ATTACK)
    }

    /// Decode from binary message bytes (after channel prefix).
    /// Format: [keys:u8, keys2:u8, touch_x_hi:u8, touch_x_lo:u8, touch_y_hi:u8, touch_y_lo:u8, touching:u8]
    pub fn decode(data: &[u8]) -> Self {
        if data.len() < 7 {
            return Self::default();
        }
        Self {
            keys: data[0],
            keys2: data[1],
            touch_x: i16::from_be_bytes([data[2], data[3]]),
            touch_y: i16::from_be_bytes([data[4], data[5]]),
            touching: data[6] != 0,
        }
    }
}
