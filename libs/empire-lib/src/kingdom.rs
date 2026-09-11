use serde::{Deserialize, Serialize};

use crate::events::RulerDeathCause;

use crate::economy::Taxes;
use crate::random::random;
use crate::weather::Weather;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlayerTitle {
    Duke,
    Prince,
    King,
    Emperor,
}

impl PlayerTitle {
    /// The four ranks, lowest first.
    pub const ALL: [PlayerTitle; 4] = [
        PlayerTitle::Duke,
        PlayerTitle::Prince,
        PlayerTitle::King,
        PlayerTitle::Emperor,
    ];

    /// The rank above this one; `None` for the Emperor.
    pub fn next(self) -> Option<PlayerTitle> {
        Self::ALL.get(self as usize + 1).copied()
    }

    /// What a kingdom must reach to hold this title; `None` for the lowest
    /// rank, which every seigneur holds by birth.
    pub fn requirements(self) -> Option<&'static Requirements> {
        match self {
            PlayerTitle::Duke => None,
            PlayerTitle::Prince => Some(&REQUIREMENTS[0]),
            PlayerTitle::King => Some(&REQUIREMENTS[1]),
            PlayerTitle::Emperor => Some(&REQUIREMENTS[2]),
        }
    }
}

/// What a rank demands of a kingdom, per the original Empire.bas. The land
/// ratio (arpents per serf) and the palace are in tenths; the ratio must be
/// exceeded, every other figure reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirements {
    pub peasants: i32,
    pub land_ratio: i32,
    pub nobles: i32,
    pub grain_mills: i32,
    pub marketplaces: i32,
    pub foundries: i32,
    pub palaces: i32,
    /// Tenths of fortifications.
    pub fortifications: i32,
    /// Tenths of hospice.
    pub hospices: i32,
}

/// Prince, King and Emperor, in that order.
const REQUIREMENTS: [Requirements; 3] = [
    Requirements {
        peasants: 2300,
        land_ratio: 48,
        nobles: 10,
        grain_mills: 4,
        marketplaces: 8,
        foundries: 0,
        palaces: 2,
        fortifications: 1,
        hospices: 1,
    },
    Requirements {
        peasants: 2600,
        land_ratio: 50,
        nobles: 25,
        grain_mills: 6,
        marketplaces: 14,
        foundries: 1,
        palaces: 6,
        fortifications: 3,
        hospices: 3,
    },
    Requirements {
        peasants: 3100,
        land_ratio: 50,
        nobles: 40,
        grain_mills: 6,
        marketplaces: 14,
        foundries: 1,
        palaces: 10,
        fortifications: 5,
        hospices: 5,
    },
];

/// One of the figures a title is judged on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    Peasants,
    /// Arpents per serf, in tenths; the only figure that must be exceeded.
    LandRatio,
    Nobles,
    GrainMills,
    Marketplaces,
    Foundries,
    /// Tenths of the palace built.
    Palaces,
    /// Tenths of fortifications raised.
    Fortifications,
    /// Tenths of hospice built.
    Hospices,
}

impl Requirement {
    /// Every requirement, the two newest last: the brain's view reads the
    /// first seven.
    pub const ALL: [Requirement; 9] = [
        Requirement::Peasants,
        Requirement::LandRatio,
        Requirement::Nobles,
        Requirement::GrainMills,
        Requirement::Marketplaces,
        Requirement::Foundries,
        Requirement::Palaces,
        Requirement::Fortifications,
        Requirement::Hospices,
    ];

    fn need(self, r: &Requirements) -> i32 {
        match self {
            Requirement::Peasants => r.peasants,
            Requirement::LandRatio => r.land_ratio,
            Requirement::Nobles => r.nobles,
            Requirement::GrainMills => r.grain_mills,
            Requirement::Marketplaces => r.marketplaces,
            Requirement::Foundries => r.foundries,
            Requirement::Palaces => r.palaces,
            Requirement::Fortifications => r.fortifications,
            Requirement::Hospices => r.hospices,
        }
    }
}

/// Where a kingdom stands on one requirement of a title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Criterion {
    pub what: Requirement,
    pub have: i32,
    pub need: i32,
}

impl Criterion {
    pub fn met(&self) -> bool {
        match self.what {
            Requirement::LandRatio => self.have > self.need,
            _ => self.have >= self.need,
        }
    }
}

/// Bushels a mouth eats in a year on a full ration.
pub const GRAIN_PER_MOUTH: i32 = 5;
/// Bushels a soldier eats in a year on a full ration.
pub const GRAIN_PER_SOLDIER: i32 = 8;
/// Rations are set per head in tenths of a bushel: this many tenths make one.
pub const RATION_SCALE: i32 = 10;

/// How a fallen kingdom fell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fate {
    /// The ruler died and the realm broke up.
    RulerDied(RulerDeathCause),
    /// Its last arpent taken by another realm.
    Annexed(Kingdoms),
}

/// Full state of one kingdom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kingdom {
    pub id: Kingdoms,
    pub player_name: String,
    pub is_player: bool,
    pub is_dead: bool,
    /// Why `is_dead`, once it is.
    #[serde(default)]
    pub fate: Option<Fate>,
    pub surface: i32,
    pub peasants: i32,
    pub nobles: i32,
    pub merchants: i32,
    pub soldiers: i32,
    /// 50..=150, in per cent.
    pub soldiers_efficiency: i32,
    /// Bushels per man the army was last put on, in tenths (80 = 8 bx): what
    /// recruits eat when hired and the default of the next council.
    pub soldiers_ration: i32,
    pub treasury: i32,
    pub grain_stocks: i32,
    pub grain_harvest: i32,
    /// The year's sky over this realm: every realm has its own.
    pub weather: Weather,
    /// Price of the bushel on the stall, in centimes.
    pub grain_price: i32,
    /// Bushels on the stall, set aside for sale: out of the stocks since
    /// they were listed — neither eaten, sown nor gnawed — until a neighbour
    /// buys them.
    pub grain_to_sell: i32,
    /// Bushels set aside this year with their price: they reach the stall at
    /// [`Kingdom::open_market`], next year.
    pub listing: Option<(i32, i32)>,
    pub rats_loss_rate: i32,
    pub marketplaces: i32,
    pub grain_mills: i32,
    pub foundries: i32,
    pub shipyards: i32,
    /// Palace completion in tenths (10 = 100%).
    pub palaces: i32,
    /// Fortifications in tenths: each adds a tenth to the garrison's
    /// strength behind its walls; rams knock them down for good.
    #[serde(default)]
    pub fortifications: i32,
    /// Hospice in tenths: each spares a twentieth of the disease and plague
    /// dead.
    #[serde(default)]
    pub hospices: i32,
    /// Rams in stock, taken along on an expedition against a kingdom.
    #[serde(default)]
    pub rams: i32,
    /// Scouts in reserve, each good for one mission.
    #[serde(default)]
    pub scouts: i32,
    pub immigration_taxes: i32,
    pub commercial_taxes: i32,
    pub income_taxes: i32,
}

/// `amount` bushels at `price` added to a `(bushels, price)` lot: the price
/// becomes the average weighted by volume.
fn merge_lot((have, have_price): (i32, i32), amount: i32, price: i32) -> (i32, i32) {
    let total = have + amount;
    let avg = if total > 0 {
        (have_price * have + price * amount) / total
    } else {
        price
    };
    (total, avg)
}

impl Kingdom {
    /// A fresh kingdom with the original Empire.bas starting values.
    pub fn new(id: Kingdoms) -> Self {
        Kingdom {
            id,
            player_name: id.default_king_name().to_string(),
            is_player: false,
            is_dead: false,
            fate: None,
            surface: 10000,
            peasants: 2000,
            nobles: 1,
            merchants: 25,
            soldiers: 20,
            soldiers_efficiency: 150,
            soldiers_ration: GRAIN_PER_SOLDIER * RATION_SCALE,
            treasury: 1000,
            // Original: A(I,2)=15000+INT(RND*1000)+1 = [15001, 16000]
            grain_stocks: 15000 + random(1, 1001),
            grain_harvest: 0,
            weather: Weather::default(),
            grain_price: 0,
            grain_to_sell: 0,
            listing: None,
            rats_loss_rate: 10,
            marketplaces: 0,
            grain_mills: 0,
            foundries: 0,
            shipyards: 0,
            palaces: 0,
            fortifications: 0,
            hospices: 0,
            rams: 0,
            scouts: 0,
            immigration_taxes: 20,
            commercial_taxes: 8,
            income_taxes: 20,
        }
    }

    /// Bushels a neighbour can buy now: what is on the stall.
    pub fn for_sale(&self) -> i32 {
        self.grain_to_sell.max(0)
    }

    /// Bushels set aside for sale, on the stall or listed for next year.
    pub fn offered(&self) -> i32 {
        self.grain_to_sell + self.listing.map_or(0, |(amount, _)| amount)
    }

    /// Set `amount` bushels aside for sale at `price` the bushel (weighted-
    /// average price if some grain was already listed this year). The grain
    /// leaves the stocks at once — reserved for the sale, it feeds nobody
    /// this year — so the lot is capped to the stocks.
    pub fn list_grain(&mut self, amount: i32, price: i32) {
        let amount = amount.min(self.grain_stocks).max(0);
        if amount == 0 {
            return;
        }
        self.grain_stocks -= amount;
        self.listing = Some(merge_lot(self.listing.unwrap_or((0, price)), amount, price));
    }

    /// Move the year's listing onto the stall, joining what did not sell —
    /// original formula A(K,6)=(A(K,6)*A(K,5)+H1*H2)/(A(K,5)+H1), a
    /// weighted-average price.
    pub fn open_market(&mut self) {
        if let Some((amount, price)) = self.listing.take() {
            let (total, avg) = merge_lot((self.grain_to_sell, self.grain_price), amount, price);
            self.grain_to_sell = total;
            self.grain_price = avg;
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

    /// Arpents per serf, in tenths.
    pub fn land_ratio(&self) -> i32 {
        if self.peasants > 0 {
            self.surface * 10 / self.peasants
        } else {
            0
        }
    }

    fn have(&self, what: Requirement) -> i32 {
        match what {
            Requirement::Peasants => self.peasants,
            Requirement::LandRatio => self.land_ratio(),
            Requirement::Nobles => self.nobles,
            Requirement::GrainMills => self.grain_mills,
            Requirement::Marketplaces => self.marketplaces,
            Requirement::Foundries => self.foundries,
            Requirement::Palaces => self.palaces,
            Requirement::Fortifications => self.fortifications,
            Requirement::Hospices => self.hospices,
        }
    }

    /// The kingdom against each requirement of `title`, in a fixed order,
    /// skipping the figures the rank does not ask for. Empty for the lowest
    /// rank.
    pub fn progress(&self, title: PlayerTitle) -> Vec<Criterion> {
        let Some(r) = title.requirements() else {
            return Vec::new();
        };
        Requirement::ALL
            .into_iter()
            .map(|what| Criterion {
                what,
                have: self.have(what),
                need: what.need(r),
            })
            .filter(|c| c.need > 0)
            .collect()
    }

    /// The realm falls: no land, no people, no ruler.
    pub fn fall(&mut self, fate: Fate) {
        self.is_dead = true;
        self.fate = Some(fate);
    }

    /// Current title: the highest rank whose every requirement is met. It is
    /// judged afresh each time, so a title can be lost — until the imperial
    /// crown, which ends the game.
    pub fn title(&self) -> PlayerTitle {
        PlayerTitle::ALL
            .into_iter()
            .rev()
            .find(|&t| self.progress(t).iter().all(Criterion::met))
            .unwrap_or(PlayerTitle::Duke)
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

    /// Mouths to feed among the people: a noble eats for three.
    pub fn mouths(&self) -> i32 {
        self.peasants + self.merchants + self.nobles * 3
    }

    pub fn peasants_grain_needs(&self) -> i32 {
        self.mouths() * GRAIN_PER_MOUTH
    }

    pub fn soldiers_grain_needs(&self) -> i32 {
        self.soldiers * GRAIN_PER_SOLDIER
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
        k.fortifications = 1;
        k.hospices = 1;
        assert_eq!(k.title(), PlayerTitle::Prince);

        k.peasants = 2600;
        k.surface = 2600 * 6;
        k.nobles = 25;
        k.grain_mills = 6;
        k.marketplaces = 14;
        k.foundries = 1;
        k.palaces = 6;
        k.fortifications = 3;
        k.hospices = 3;
        assert_eq!(k.title(), PlayerTitle::King);

        k.peasants = 3100;
        k.surface = 3100 * 6;
        k.nobles = 40;
        k.palaces = 10;
        k.fortifications = 5;
        k.hospices = 5;
        assert_eq!(k.title(), PlayerTitle::Emperor);
        assert_eq!(k.titled_name(), "Empereur Hugues");
        assert_eq!(k.progress(PlayerTitle::Emperor).len(), 9);
        assert!(k.progress(PlayerTitle::Emperor).iter().all(Criterion::met));
    }

    #[test]
    fn ranks_climb_and_end_at_the_emperor() {
        assert_eq!(PlayerTitle::Duke.next(), Some(PlayerTitle::Prince));
        assert_eq!(PlayerTitle::King.next(), Some(PlayerTitle::Emperor));
        assert_eq!(PlayerTitle::Emperor.next(), None);
        assert!(PlayerTitle::Duke < PlayerTitle::Emperor);
        assert!(PlayerTitle::Duke.requirements().is_none());
    }

    #[test]
    fn a_fallen_realm_keeps_its_fate() {
        let mut k = Kingdom::new(Kingdoms::France);
        assert_eq!(k.fate, None);
        k.fall(Fate::Annexed(Kingdoms::Spain));
        assert!(k.is_dead);
        assert_eq!(k.fate, Some(Fate::Annexed(Kingdoms::Spain)));
    }

    #[test]
    fn progress_shows_what_is_missing() {
        let k = Kingdom::new(Kingdoms::France);
        assert!(k.progress(PlayerTitle::Duke).is_empty());
        let p = k.progress(PlayerTitle::Prince);
        // The Prince asks nothing of the foundry.
        assert_eq!(p.len(), 8);
        assert!(!p.iter().any(|c| c.what == Requirement::Foundries));
        let serfs = p.iter().find(|c| c.what == Requirement::Peasants).unwrap();
        assert_eq!((serfs.have, serfs.need, serfs.met()), (2000, 2300, false));
        let ratio = p.iter().find(|c| c.what == Requirement::LandRatio).unwrap();
        assert_eq!((ratio.have, ratio.need), (k.land_ratio(), 48));
        // The land ratio must be exceeded, not merely reached.
        assert!(!Criterion {
            what: Requirement::LandRatio,
            have: 48,
            need: 48
        }
        .met());
        assert!(Criterion {
            what: Requirement::LandRatio,
            have: 49,
            need: 48
        }
        .met());
        assert!(Criterion {
            what: Requirement::Nobles,
            have: 10,
            need: 10
        }
        .met());
    }

    #[test]
    fn a_title_is_the_rank_whose_every_criterion_is_met() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.peasants = 2600;
        k.surface = 2600 * 6;
        k.nobles = 25;
        k.grain_mills = 6;
        k.marketplaces = 14;
        k.foundries = 1;
        k.palaces = 6;
        k.fortifications = 3;
        k.hospices = 3;
        assert_eq!(k.title(), PlayerTitle::King);
        // One criterion lapsing takes the title away.
        k.foundries = 0;
        assert_eq!(k.title(), PlayerTitle::Prince);
        assert_eq!(
            k.progress(PlayerTitle::King)
                .iter()
                .filter(|c| !c.met())
                .map(|c| c.what)
                .collect::<Vec<_>>(),
            [Requirement::Foundries]
        );
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
