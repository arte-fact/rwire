use crate::ui::{bottom_press_enter, clear_screen, print_at};

pub fn welcome() {
    clear_screen();
    print_at(9, 35, "E M P I R E");
    bottom_press_enter();
}
