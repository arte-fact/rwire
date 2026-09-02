use serde::{Deserialize, Serialize};

use crate::economy::Taxes;
use crate::random::random;

pub const KINGDOMS: [Kingdoms; 6] = [
    Kingdoms::France,
    Kingdoms::Britanny,
    Kingdoms::Germany,
    Kingdoms::Spain,
    Kingdoms::Moscovy,
    Kingdoms::Persia,
];

/// Identifier of one of the six kingdoms. The per-kingdom data lives in
/// [`Kingdom`], indexed by this id inside [`crate::EmpireGame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kingdoms {
    France,
    Britanny,
    Germany,
    Spain,
    Moscovy,
    Persia,
}

impl Kingdoms {
    /// Position in [`KINGDOMS`] / `EmpireGame::kingdoms`.
    pub fn index(self) -> usize {
        self as usize
    }

    /// 1-based lookup (1 = France … 6 = Persia), as used by the UI menus.
    pub fn from_number(n: i32) -> Option<Kingdoms> {
        let idx = usize::try_from(n.checked_sub(1)?).ok()?;
        KINGDOMS.get(idx).copied()
    }

    pub fn name(self) -> &'static str {
        match self {
            Kingdoms::France => "France",
            Kingdoms::Britanny => "Bretagne",
            Kingdoms::Germany => "Germanie",
            Kingdoms::Spain => "Castille",
            Kingdoms::Moscovy => "Moscovie",
            Kingdoms::Persia => "Perse",
        }
    }

    pub fn currency(self) -> &'static str {
        match self {
            Kingdoms::France => "francs",
            Kingdoms::Britanny => "livres",
            Kingdoms::Germany => "marcs",
            Kingdoms::Spain => "ecus",
            Kingdoms::Persia => "dinars",
            Kingdoms::Moscovy => "roubles",
        }
    }

    pub fn default_king_name(self) -> &'static str {
        match self {
            Kingdoms::France => "Hugues",
            Kingdoms::Britanny => "Arthur",
            Kingdoms::Germany => "Othon",
            Kingdoms::Spain => "Rodrigue",
            Kingdoms::Persia => "Kubeni",
            Kingdoms::Moscovy => "Ivan",
        }
    }

    /// Localised name of a title for this kingdom.
    pub fn title_name(self, title: PlayerTitle) -> &'static str {
        match self {
            Kingdoms::France => match title {
                PlayerTitle::Duke => "Baron",
                PlayerTitle::Prince => "Duc",
                PlayerTitle::King => "Roy",
                PlayerTitle::Emperor => "Empereur",
            },
            Kingdoms::Britanny => match title {
                PlayerTitle::Duke => "Sir",
                PlayerTitle::Prince => "Prince",
                PlayerTitle::King => "King",
                PlayerTitle::Emperor => "Emperor",
            },
            Kingdoms::Germany => match title {
                PlayerTitle::Duke => "Chevalier",
                PlayerTitle::Prince => "Prinz",
                PlayerTitle::King => "König",
                PlayerTitle::Emperor => "Kaiser",
            },
            Kingdoms::Spain => match title {
                PlayerTitle::Duke => "Don",
                PlayerTitle::Prince => "Príncipe",
                PlayerTitle::King => "Rey",
                PlayerTitle::Emperor => "Emperador",
            },
            Kingdoms::Moscovy => match title {
                PlayerTitle::Duke => "Boyar",
                PlayerTitle::Prince => "Knyaz",
                PlayerTitle::King => "Tsar",
                PlayerTitle::Emperor => "Imperator",
            },
            Kingdoms::Persia => match title {
                PlayerTitle::Duke => "Ayatollah",
                PlayerTitle::Prince => "Shahzadeh",
                PlayerTitle::King => "Sultan",
                PlayerTitle::Emperor => "Empereur",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerTitle {
    Duke,
    Prince,
    King,
    Emperor,
}

/// Full state of one kingdom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kingdom {
    pub id: Kingdoms,
    pub player_name: String,
    pub is_player: bool,
    pub is_dead: bool,
    pub surface: i32,
    pub peasants: i32,
    pub nobles: i32,
    pub merchants: i32,
    pub soldiers: i32,
    /// 5..=15, displayed as 50%..150%.
    pub soldiers_efficiency: i32,
    pub treasury: i32,
    pub grain_stocks: i32,
    pub grain_harvest: i32,
    pub grain_price: i32,
    pub grain_to_sell: i32,
    pub rats_loss_rate: i32,
    pub marketplaces: i32,
    pub grain_mills: i32,
    pub foundries: i32,
    pub shipyards: i32,
    /// Palace completion in tenths (10 = 100%).
    pub palaces: i32,
    pub immigration_taxes: i32,
    pub commercial_taxes: i32,
    pub income_taxes: i32,
}

impl Kingdom {
    /// A fresh kingdom with the original Empire.bas starting values.
    pub fn new(id: Kingdoms) -> Self {
        Kingdom {
            id,
            player_name: id.default_king_name().to_string(),
            is_player: false,
            is_dead: false,
            surface: 10000,
            peasants: 2000,
            nobles: 1,
            merchants: 25,
            soldiers: 20,
            soldiers_efficiency: 150,
            treasury: 1000,
            // Original: A(I,2)=15000+INT(RND*1000)+1 = [15001, 16000]
            grain_stocks: 15000 + random(1, 1001),
            grain_harvest: 0,
            grain_price: 0,
            grain_to_sell: 0,
            rats_loss_rate: 10,
            marketplaces: 0,
            grain_mills: 0,
            foundries: 0,
            shipyards: 0,
            palaces: 0,
            immigration_taxes: 20,
            commercial_taxes: 8,
            income_taxes: 20,
        }
    }

    pub fn taxes(&self) -> Taxes {
        Taxes {
            customs: self.immigration_taxes,
            sales: self.commercial_taxes,
            income: self.income_taxes,
        }
    }

    pub fn name(&self) -> &'static str {
        self.id.name()
    }

    pub fn currency(&self) -> &'static str {
        self.id.currency()
    }

    /// Current title per original Empire.bas requirements:
    /// - Prince: marketplaces >= 8, mills >= 4, palace >= 20%, land/serfs > 4.8, nobles >= 10, serfs >= 2,300
    /// - King: marketplaces >= 14, mills >= 6, foundries >= 1, palace >= 60%, land/serfs > 5.0, nobles >= 25, serfs >= 2,600
    /// - Emperor: all King requirements + palace 100%, nobles >= 40, serfs >= 3,100
    pub fn title(&self) -> PlayerTitle {
        // Land/serfs ratio x10 for integer precision.
        let land_ratio = if self.peasants > 0 {
            self.surface * 10 / self.peasants
        } else {
            0
        };

        if self.peasants >= 3100
            && land_ratio > 50
            && self.nobles >= 40
            && self.grain_mills >= 6
            && self.marketplaces >= 14
            && self.foundries >= 1
            && self.palaces >= 10
        {
            return PlayerTitle::Emperor;
        }

        if self.peasants >= 2600
            && land_ratio > 50
            && self.nobles >= 25
            && self.grain_mills >= 6
            && self.marketplaces >= 14
            && self.foundries >= 1
            && self.palaces >= 6
        {
            return PlayerTitle::King;
        }

        if self.peasants >= 2300
            && land_ratio > 48
            && self.nobles >= 10
            && self.grain_mills >= 4
            && self.marketplaces >= 8
            && self.palaces >= 2
        {
            return PlayerTitle::Prince;
        }

        PlayerTitle::Duke
    }

    pub fn title_name(&self) -> &'static str {
        self.id.title_name(self.title())
    }

    /// "Roy Hugues"
    pub fn titled_name(&self) -> String {
        format!("{} {}", self.title_name(), self.player_name)
    }

    /// "Roy Hugues de France"
    pub fn full_title(&self) -> String {
        format!(
            "{} {} de {}",
            self.title_name(),
            self.player_name,
            self.name()
        )
    }

    pub fn peasants_grain_needs(&self) -> i32 {
        (self.peasants + self.merchants + self.nobles * 3) * 5
    }

    pub fn soldiers_grain_needs(&self) -> i32 {
        self.soldiers * 8
    }

    pub fn total_population(&self) -> i32 {
        self.peasants + self.nobles + self.merchants + self.soldiers
    }

    /// Civilian population (serfs + nobles + merchants).
    pub fn population(&self) -> i32 {
        self.peasants + self.nobles + self.merchants
    }

    pub fn cultivated_surface(&self) -> i32 {
        self.surface
            - self.peasants
            - self.nobles * 2
            - self.palaces
            - self.merchants
            - self.soldiers * 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_kingdom_has_original_starting_values() {
        let k = Kingdom::new(Kingdoms::Spain);
        assert_eq!(k.player_name, "Rodrigue");
        assert_eq!(k.surface, 10000);
        assert_eq!(k.peasants, 2000);
        assert!((15001..=16000).contains(&k.grain_stocks));
        assert_eq!(k.soldiers_efficiency, 150);
        assert!(!k.is_player && !k.is_dead);
    }

    #[test]
    fn index_and_from_number_round_trip() {
        for (i, id) in KINGDOMS.iter().enumerate() {
            assert_eq!(id.index(), i);
            assert_eq!(Kingdoms::from_number(i as i32 + 1), Some(*id));
        }
        assert_eq!(Kingdoms::from_number(0), None);
        assert_eq!(Kingdoms::from_number(7), None);
        assert_eq!(Kingdoms::from_number(-3), None);
    }

    #[test]
    fn titles_follow_thresholds() {
        let mut k = Kingdom::new(Kingdoms::France);
        assert_eq!(k.title(), PlayerTitle::Duke);
        assert_eq!(k.full_title(), "Baron Hugues de France");

        k.peasants = 2300;
        k.surface = 2300 * 5;
        k.nobles = 10;
        k.grain_mills = 4;
        k.marketplaces = 8;
        k.palaces = 2;
        assert_eq!(k.title(), PlayerTitle::Prince);

        k.peasants = 2600;
        k.surface = 2600 * 6;
        k.nobles = 25;
        k.grain_mills = 6;
        k.marketplaces = 14;
        k.foundries = 1;
        k.palaces = 6;
        assert_eq!(k.title(), PlayerTitle::King);

        k.peasants = 3100;
        k.surface = 3100 * 6;
        k.nobles = 40;
        k.palaces = 10;
        assert_eq!(k.title(), PlayerTitle::Emperor);
        assert_eq!(k.titled_name(), "Empereur Hugues");
    }

    #[test]
    fn derived_quantities() {
        let k = Kingdom::new(Kingdoms::France);
        assert_eq!(k.peasants_grain_needs(), (2000 + 25 + 3) * 5);
        assert_eq!(k.soldiers_grain_needs(), 160);
        assert_eq!(k.population(), 2026);
        assert_eq!(k.total_population(), 2046);
        assert_eq!(k.cultivated_surface(), 10000 - 2000 - 2 - 25 - 40);
    }
}
