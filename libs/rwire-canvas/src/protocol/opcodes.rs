//! Canvas 2D binary opcode constants.
//!
//! Single-byte opcodes for streaming Canvas 2D draw commands from server to browser.
//! Coordinates use i16/u16 (sufficient for 10240×10240px worlds).

// Frame control (0x00-0x0F)
pub const FRAME_BEGIN: u8 = 0x00; // tick:u32
pub const FRAME_END: u8 = 0x01;
pub const CLEAR: u8 = 0x02;

// Transform stack (0x10-0x1F)
pub const SAVE: u8 = 0x10;
pub const RESTORE: u8 = 0x11;
pub const TRANSLATE: u8 = 0x12; // x:i16, y:i16
pub const SCALE_XY: u8 = 0x13; // sx:u16, sy:u16 (fixed-point: value/256)
pub const ROTATE: u8 = 0x14; // angle:u16 (radians = value/10430.378)

// Draw primitives (0x20-0x2F)
pub const FILL_RECT: u8 = 0x20; // x:i16, y:i16, w:u16, h:u16
pub const STROKE_RECT: u8 = 0x21; // x:i16, y:i16, w:u16, h:u16
pub const BEGIN_PATH: u8 = 0x23;
pub const MOVE_TO: u8 = 0x24; // x:i16, y:i16
pub const LINE_TO: u8 = 0x25; // x:i16, y:i16
pub const ARC: u8 = 0x26; // cx:i16, cy:i16, r:u16, start:u16, end:u16
pub const FILL: u8 = 0x27;
pub const STROKE: u8 = 0x28;
pub const CLOSE_PATH: u8 = 0x29;
pub const ROUND_RECT: u8 = 0x2A; // x:i16, y:i16, w:u16, h:u16, r:u8

// Image operations (0x30-0x3F)
pub const DRAW_IMAGE: u8 = 0x30; // tex:u8, sx:u16, sy:u16, sw:u16, sh:u16, dx:i16, dy:i16, dw:u16, dh:u16
pub const DRAW_IMAGE_SIMPLE: u8 = 0x31; // tex:u8, dx:i16, dy:i16, dw:u16, dh:u16
pub const DRAW_SPRITE: u8 = 0x32; // sprite_id:u16, dx:i16, dy:i16

// Text operations (0x40-0x4F)
pub const FILL_TEXT: u8 = 0x40; // x:i16, y:i16, len:u8, utf8_bytes...
pub const STROKE_TEXT: u8 = 0x41; // x:i16, y:i16, len:u8, utf8_bytes...

// Style state (0x50-0x5F)
pub const SET_FILL_RGBA: u8 = 0x50; // r:u8, g:u8, b:u8, a:u8
pub const SET_FILL_IDX: u8 = 0x51; // color_idx:u8
pub const SET_STROKE_RGBA: u8 = 0x52; // r:u8, g:u8, b:u8, a:u8
pub const SET_STROKE_IDX: u8 = 0x53; // color_idx:u8
pub const SET_ALPHA: u8 = 0x54; // alpha:u8 (0-255, mapped to 0.0-1.0)
pub const SET_LINE_WIDTH: u8 = 0x55; // width:u8 (quarter pixels: value/4)
pub const SET_LINE_DASH: u8 = 0x56; // count:u8, segments:u8...
pub const SET_FONT: u8 = 0x57; // font_idx:u8
pub const SET_TEXT_ALIGN: u8 = 0x58; // 0=left, 1=center, 2=right
pub const SET_TEXT_BASELINE: u8 = 0x59; // 0=top, 1=middle, 2=bottom, 3=alphabetic
pub const SET_SMOOTHING: u8 = 0x5A; // 0=off, 1=on
pub const SET_COMPOSITE: u8 = 0x5B; // 0=source-over, 1=multiply, 2=screen

// Metadata / tables (0xF0-0xFF)
pub const TEXTURE_TABLE: u8 = 0xF0; // count:u8, [id:u8, path_len:u8, path_utf8...]×n
pub const COLOR_TABLE: u8 = 0xF1; // count:u8, [r:u8, g:u8, b:u8, a:u8]×n
pub const FONT_TABLE: u8 = 0xF2; // count:u8, [len:u8, font_str_utf8...]×n
pub const SPRITE_TABLE: u8 = 0xF3; // count_hi:u8, count_lo:u8, [tex:u8, sx:u16, sy:u16, sw:u16, sh:u16]×n
pub const FOG_GRID: u8 = 0xF8; // x:u16, y:u16, w:u16, h:u16, tile_size:u8, [alpha:u8]×(w*h) — fog overlay grid
pub const ENTITY_BATCH: u8 = 0xF9; // count_hi:u8, count_lo:u8, [id:u16, x:i16, y:i16, sprite:u16, flags:u8]×n
