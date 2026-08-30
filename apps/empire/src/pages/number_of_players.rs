use crate::assets::text;
use crate::ui::{clear_screen, input_number_with_range_at, print_at};

pub fn prompt_number_of_players() -> usize {
    let top = 9;
    clear_screen();
    print_at(top, 28, text::HOW_MANY_PLAYERS);
    print_at(top + 4, 28, &format!("{} : 1", text::DEFAULT_IF_EMPTY));
    input_number_with_range_at(top + 2, 28, 1, 6, None).unwrap_or(1)
}
