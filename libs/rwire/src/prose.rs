//! `ProseFace`: the face a block of prose is set in — the reading face
//! (`--Qfr`, long-form answers) or the interface face.

use crate::builder::ElementBuilder;
use crate::style_tokens::St;

/// The face of a block of prose. `Read`: the theme's reading face (`--Qfr`,
/// `St::FontRead`), 15 px on a phone and 15.5 px from `sm`, line height 1.6
/// (an agent's answer). `Ui`: the interface face, as inherited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProseFace {
    /// The reading face, 15 / 15.5 px, line height 1.6.
    #[default]
    Read,
    /// The interface face (inherited).
    Ui,
}

impl ProseFace {
    /// Set `el` in this face.
    pub fn apply(self, el: ElementBuilder) -> ElementBuilder {
        match self {
            ProseFace::Read => el
                .st([St::FontRead, St::TextProsePhone])
                .sm([St::TextProse]),
            ProseFace::Ui => el,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{el, El};

    #[test]
    fn read_is_the_reading_face_at_15_and_15_5px() {
        let p = ProseFace::Read.apply(el(El::Div));
        let utils = p.get_style_utils();
        assert!(utils.contains(&St::FontRead.as_u16()));
        assert!(utils.contains(&St::TextProsePhone.as_u16()));
        let sm: Vec<u16> = p
            .get_breakpoint_groups()
            .iter()
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        assert!(sm.contains(&St::TextProse.as_u16()));
        assert!(ProseFace::Ui
            .apply(el(El::Div))
            .get_style_utils()
            .is_empty());
        assert_eq!(St::TextProsePhone.css(), "font-size:15px;line-height:1.6");
    }
}
