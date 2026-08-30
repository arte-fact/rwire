use crate::assets::text;
use crate::ui::{clear_screen, move_cursor_to, print_at, read_line};

pub fn prompt_player_name(land_name: &str, default: &str) -> String {
    let top = 9;
    clear_screen();

    print_at(
        top,
        28,
        &format!("{} {} ?", text::ENTER_PLAYER_NAME, land_name),
    );
    print_at(
        top + 4,
        28,
        &format!("{} : {}", text::DEFAULT_IF_EMPTY, default),
    );

    move_cursor_to(top + 2, 28);

    let trimmed = read_line().trim().to_string();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed
    }
}
