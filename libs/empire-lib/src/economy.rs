use crate::demography::Outlook;
use crate::kingdom::Kingdom;
use crate::random::random;
use crate::weather::Weather;

/// The three tax rates, in per cent, each capped as in the original
/// Empire.bas: customs 50, sales 20, income 35.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Taxes {
    /// Customs on the immigrants ("droits de douane").
    pub customs: i32,
    /// Sales tax on the trade ("gabelle").
    pub sales: i32,
    /// Direct tax on the wealth ("taille").
    pub income: i32,
}

impl Taxes {
    pub const MAX_CUSTOMS: i32 = 50;
    pub const MAX_SALES: i32 = 20;
    pub const MAX_INCOME: i32 = 35;

    pub fn clamped(self) -> Taxes {
        Taxes {
            customs: self.customs.clamp(0, Self::MAX_CUSTOMS),
            sales: self.sales.clamp(0, Self::MAX_SALES),
            income: self.income.clamp(0, Self::MAX_INCOME),
        }
    }

    /// Each rate as a share of its cap, 0–1: the curves' `x`, `y`, `z`.
    pub fn shares(self) -> (f32, f32, f32) {
        let t = self.clamped();
        (
            t.customs as f32 / Self::MAX_CUSTOMS as f32,
            t.sales as f32 / Self::MAX_SALES as f32,
            t.income as f32 / Self::MAX_INCOME as f32,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxType {
    Immigration,
    Commercial,
    Income,
}

impl TaxType {
    pub fn max_rate(self) -> i32 {
        match self {
            TaxType::Immigration => Taxes::MAX_CUSTOMS,
            TaxType::Commercial => Taxes::MAX_SALES,
            TaxType::Income => Taxes::MAX_INCOME,
        }
    }

    pub fn from_number(n: i32) -> Option<TaxType> {
        match n {
            1 => Some(TaxType::Immigration),
            2 => Some(TaxType::Commercial),
            3 => Some(TaxType::Income),
            _ => None,
        }
    }
}

/// Set a tax rate, clamped to `[0, max_rate]`.
pub fn apply_tax_change(kingdom: &mut Kingdom, tax_type: TaxType, rate: i32) {
    let rate = rate.clamp(0, tax_type.max_rate());
    match tax_type {
        TaxType::Immigration => kingdom.immigration_taxes = rate,
        TaxType::Commercial => kingdom.commercial_taxes = rate,
        TaxType::Income => kingdom.income_taxes = rate,
    }
}

/// Set the three rates at once, each clamped.
pub fn apply_taxes(kingdom: &mut Kingdom, taxes: Taxes) {
    let t = taxes.clamped();
    kingdom.immigration_taxes = t.customs;
    kingdom.commercial_taxes = t.sales;
    kingdom.income_taxes = t.income;
}

#[derive(Debug, Clone)]
pub struct YearEconomy {
    pub marketplaces_profits: i32,
    pub grain_mills_profits: i32,
    pub foundries_profits: i32,
    pub shipyards_profits: i32,
    pub soldiers_maintenance: i32,
    pub immigration_taxes_profits: i32,
    pub commercial_taxes_profits: i32,
    pub income_taxes_profits: i32,
}

impl YearEconomy {
    pub fn net(&self) -> i32 {
        self.marketplaces_profits
            + self.grain_mills_profits
            + self.shipyards_profits
            + self.foundries_profits
            + self.income_taxes_profits
            + self.commercial_taxes_profits
            + self.immigration_taxes_profits
            - self.soldiers_maintenance
    }
}

pub fn apply_economy(kingdom: &mut Kingdom, eco: &YearEconomy) {
    kingdom.treasury += eco.net();
}

// The revenue follows Empire.bas lines 96–98, with two divisors softened:
// the original fairs lost five sixths of their trade at the first point of
// gabelle, and an untaxed mill earned fifteen thousand écus a year — a kingdom
// at 0/0/0 out-earned any honest treasury. The rates themselves weigh on the
// people (births, the court, the merchants) in `demography`, not here.

/// Customs: each immigrant pays `rate × 0.4` écus.
const CUSTOMS_PER_HEAD: f32 = 0.4;
/// Original: the fairs' trade is divided by `sales + 1`; `+ 2` halves the
/// untaxed windfall and keeps the first points of gabelle from gutting them.
const FAIR_TAX_OFFSET: f32 = 2.0;
/// Original: the mills' flour is divided by `income × 20 + sales × 40 + 10`;
/// `+ 200` keeps the untaxed mill within reach of the taxed one.
const MILL_TAX_OFFSET: f32 = 200.0;

/// Both dice of the fairs: 2 × random(1, 35) → 2..68.
const FAIR_DICE: (i32, i32) = (2, 68);
/// The mills' die: random(1, 250) → 1..249.
const MILL_DIE: (i32, i32) = (1, 249);
/// The foundries' die: random(1, 150) → 1..149.
const FOUNDRY_DIE: (i32, i32) = (1, 149);

/// Original: F1 = (marketplaces × ((merchants + 2d35) / (sales + 1) × 12 + 5))^0.9
fn marketplaces_profits(kingdom: &Kingdom, taxes: Taxes, dice: i32) -> f32 {
    let trade = (kingdom.merchants + dice) as f32 / (taxes.sales as f32 + FAIR_TAX_OFFSET);
    (kingdom.marketplaces as f32 * (trade * 12.0 + 5.0)).powf(0.9)
}

/// Original: F2 = (mills × (5.8 × (harvest + d250) / (income × 20 + sales × 40 + 10) + 150))^0.9
fn grain_mills_profits(kingdom: &Kingdom, taxes: Taxes, die: i32) -> f32 {
    let divisor = taxes.income as f32 * 20.0 + taxes.sales as f32 * 40.0 + MILL_TAX_OFFSET;
    let flour = 5.8 * (kingdom.grain_harvest + die) as f32 / divisor;
    (kingdom.grain_mills as f32 * (flour + 150.0)).powf(0.9)
}

/// Original: F3 = (foundries + (soldiers + d150 + 400))^0.9 — the crown's
/// own arms trade, taxed by nothing.
fn foundries_profits(kingdom: &Kingdom, die: i32) -> f32 {
    ((kingdom.foundries + kingdom.soldiers + die + 400) as f32).powf(0.9)
}

/// Original: F4 = (shipyards × (merchants × 4 + marketplaces × 9 + foundries × 15) × weather)^0.9
fn shipyards_profits(kingdom: &Kingdom, weather: Weather) -> f32 {
    let cargo = kingdom.merchants as f32 * 4.0
        + kingdom.marketplaces as f32 * 9.0
        + kingdom.foundries as f32 * 15.0;
    (kingdom.shipyards as f32 * cargo * weather.value() as f32).powf(0.9)
}

fn soldiers_maintenance(kingdom: &Kingdom) -> i32 {
    kingdom.soldiers * 8
}

/// Original: FC = DE × (2d40) / 100 × rate — the dice are gone, every
/// immigrant pays the same.
fn customs_profits(immigrants: i32, taxes: Taxes) -> i32 {
    (immigrants as f32 * CUSTOMS_PER_HEAD * taxes.customs as f32).round() as i32
}

/// Original: FS = rate / 100 × ((merchants × 1.8 + F1 × 33 + F2 × 17 + F3 × 50 +
/// F4 × 70)^0.85 + nobles × 5 + peasants) — levied on the year's trade, the
/// foundries and the shipyards weighing most, plus a head tax.
fn commercial_taxes_profits(kingdom: &Kingdom, taxes: Taxes, trade: [f32; 4]) -> i32 {
    let [fairs, mills, foundries, ships] = trade;
    let goods = kingdom.merchants as f32 * 1.8
        + fairs * 33.0
        + mills * 17.0
        + foundries * 50.0
        + ships * 70.0;
    let base = goods.powf(0.85) + kingdom.nobles as f32 * 5.0 + kingdom.peasants as f32;
    (taxes.sales as f32 / 100.0 * base) as i32
}

/// Original: FI = (rate / 100 × (peasants × 1.3 + nobles × 145 + merchants × 39
/// + marketplaces × 99 + mills × 99 + foundries × 425 + shipyards × 965))^0.97
fn income_taxes_profits(kingdom: &Kingdom, taxes: Taxes) -> i32 {
    let wealth = kingdom.peasants as f32 * 1.3
        + kingdom.nobles as f32 * 145.0
        + kingdom.merchants as f32 * 39.0
        + kingdom.marketplaces as f32 * 99.0
        + kingdom.grain_mills as f32 * 99.0
        + kingdom.foundries as f32 * 425.0
        + kingdom.shipyards as f32 * 965.0;
    (taxes.income as f32 / 100.0 * wealth).powf(0.97) as i32
}

/// The year's economy for a given set of dice.
fn economy_with(
    kingdom: &Kingdom,
    weather: Weather,
    immigrants: i32,
    taxes: Taxes,
    fair_dice: i32,
    mill_die: i32,
    foundry_die: i32,
) -> YearEconomy {
    let trade = [
        marketplaces_profits(kingdom, taxes, fair_dice),
        grain_mills_profits(kingdom, taxes, mill_die),
        foundries_profits(kingdom, foundry_die),
        shipyards_profits(kingdom, weather),
    ];
    YearEconomy {
        marketplaces_profits: trade[0] as i32,
        grain_mills_profits: trade[1] as i32,
        foundries_profits: trade[2] as i32,
        shipyards_profits: trade[3] as i32,
        soldiers_maintenance: soldiers_maintenance(kingdom),
        immigration_taxes_profits: customs_profits(immigrants, taxes),
        commercial_taxes_profits: commercial_taxes_profits(kingdom, taxes, trade),
        income_taxes_profits: income_taxes_profits(kingdom, taxes),
    }
}

/// The year's economy under the kingdom's current rates, dice rolled.
pub fn economy_report(kingdom: &Kingdom, weather: Weather, immigrants: i32) -> YearEconomy {
    economy_with(
        kingdom,
        weather,
        immigrants,
        kingdom.taxes(),
        random(1, 35) + random(1, 35),
        random(1, 250),
        random(1, 150),
    )
}

/// What the year should bring to the treasury, as hard bounds; the dice, the
/// weather (already drawn) and the immigrants' bounds are the only
/// uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EconomyOutlook {
    pub marketplaces: Outlook,
    pub grain_mills: Outlook,
    pub foundries: Outlook,
    pub shipyards: i32,
    pub soldiers_maintenance: i32,
    pub immigration_taxes: Outlook,
    pub commercial_taxes: Outlook,
    pub income_taxes: i32,
}

impl EconomyOutlook {
    pub fn net(&self) -> Outlook {
        (self.marketplaces
            + self.grain_mills
            + self.foundries
            + self.immigration_taxes
            + self.commercial_taxes)
            .plus(self.shipyards + self.income_taxes - self.soldiers_maintenance)
    }
}

pub fn economy_outlook(
    kingdom: &Kingdom,
    weather: Weather,
    immigrants: Outlook,
    taxes: Taxes,
) -> EconomyOutlook {
    // Every figure grows with its die and with the immigrants, so the bounds
    // are the two extreme years and the marker the middle one.
    let year =
        |imm: i32, f: i32, m: i32, o: i32| economy_with(kingdom, weather, imm, taxes, f, m, o);
    let low = year(immigrants.low, FAIR_DICE.0, MILL_DIE.0, FOUNDRY_DIE.0);
    let mid = year(
        immigrants.expected,
        (FAIR_DICE.0 + FAIR_DICE.1) / 2,
        (MILL_DIE.0 + MILL_DIE.1) / 2,
        (FOUNDRY_DIE.0 + FOUNDRY_DIE.1) / 2,
    );
    let high = year(immigrants.high, FAIR_DICE.1, MILL_DIE.1, FOUNDRY_DIE.1);
    let span = |f: fn(&YearEconomy) -> i32| Outlook {
        low: f(&low),
        expected: f(&mid),
        high: f(&high),
    };
    EconomyOutlook {
        marketplaces: span(|e| e.marketplaces_profits),
        grain_mills: span(|e| e.grain_mills_profits),
        foundries: span(|e| e.foundries_profits),
        shipyards: mid.shipyards_profits,
        soldiers_maintenance: mid.soldiers_maintenance,
        immigration_taxes: span(|e| e.immigration_taxes_profits),
        commercial_taxes: span(|e| e.commercial_taxes_profits),
        income_taxes: mid.income_taxes_profits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Kingdoms;

    fn rich() -> Kingdom {
        let mut k = Kingdom::new(Kingdoms::France);
        k.merchants = 60;
        k.nobles = 12;
        k.marketplaces = 6;
        k.grain_mills = 3;
        k.foundries = 1;
        k.shipyards = 1;
        k.grain_harvest = 12_000;
        k
    }

    #[test]
    fn tax_change_is_clamped() {
        let mut k = Kingdom::new(Kingdoms::France);
        apply_tax_change(&mut k, TaxType::Commercial, 99);
        assert_eq!(k.commercial_taxes, 20);
        apply_tax_change(&mut k, TaxType::Income, -5);
        assert_eq!(k.income_taxes, 0);
        apply_tax_change(&mut k, TaxType::Immigration, 30);
        assert_eq!(k.immigration_taxes, 30);
        apply_taxes(
            &mut k,
            Taxes {
                customs: 80,
                sales: -1,
                income: 10,
            },
        );
        assert_eq!(
            k.taxes(),
            Taxes {
                customs: 50,
                sales: 0,
                income: 10
            }
        );
    }

    #[test]
    fn economy_is_applied_to_treasury() {
        let mut k = Kingdom::new(Kingdoms::France);
        let eco = economy_report(&k, Weather::Good, 0);
        assert_eq!(eco.soldiers_maintenance, 160);
        assert_eq!(eco.marketplaces_profits, 0);
        assert!(eco.income_taxes_profits > 0);
        let before = k.treasury;
        apply_economy(&mut k, &eco);
        assert_eq!(k.treasury, before + eco.net());
    }

    #[test]
    fn taxes_yield_and_drag() {
        let k = rich();
        let imm = Outlook::sure(100);
        let at = |c, s, i| {
            economy_outlook(
                &k,
                Weather::Good,
                imm,
                Taxes {
                    customs: c,
                    sales: s,
                    income: i,
                },
            )
        };
        let mild = at(20, 8, 20);
        assert_eq!(mild.immigration_taxes, Outlook::sure(800));
        assert_eq!(at(0, 8, 20).immigration_taxes, Outlook::sure(0));
        // The gabelle slows the fairs and the mills…
        assert!(at(20, 20, 20).marketplaces.expected < mild.marketplaces.expected);
        assert!(at(20, 20, 20).grain_mills.expected < mild.grain_mills.expected);
        // …but neither the foundries nor the shipyards, which it taxes instead.
        assert_eq!(at(20, 20, 20).shipyards, mild.shipyards);
        assert_eq!(at(20, 20, 20).foundries, mild.foundries);
        assert!(at(20, 20, 20).commercial_taxes.expected > mild.commercial_taxes.expected);
        assert_eq!(at(20, 0, 20).commercial_taxes, Outlook::sure(0));
        // The taille keeps growing to the cap, but less than linearly, and
        // weighs on the mills too.
        let full = at(20, 8, 35).income_taxes as f32;
        let mild_income = mild.income_taxes as f32;
        assert!(full > mild_income && full < mild_income * 35.0 / 20.0);
        assert!(at(20, 8, 35).grain_mills.expected < mild.grain_mills.expected);
        // No tax at all is no windfall: the honest rates out-earn it.
        assert!(at(0, 0, 0).net().expected < mild.net().expected);
    }

    #[test]
    fn outlook_brackets_the_report() {
        let mut k = rich();
        apply_taxes(
            &mut k,
            Taxes {
                customs: 27,
                sales: 13,
                income: 30,
            },
        );
        let imm = Outlook {
            low: 40,
            expected: 100,
            high: 160,
        };
        let o = economy_outlook(&k, Weather::Bad, imm, k.taxes());
        assert!(o.marketplaces.low < o.marketplaces.high);
        assert!(o.grain_mills.low < o.grain_mills.high);
        for _ in 0..1000 {
            let r = economy_report(&k, Weather::Bad, random(imm.low, imm.high + 1));
            assert!(
                o.marketplaces.contains(r.marketplaces_profits),
                "{o:?} {r:?}"
            );
            assert!(o.grain_mills.contains(r.grain_mills_profits), "{o:?} {r:?}");
            assert!(o.foundries.contains(r.foundries_profits), "{o:?} {r:?}");
            assert_eq!(o.shipyards, r.shipyards_profits);
            assert!(o.immigration_taxes.contains(r.immigration_taxes_profits));
            assert!(o.commercial_taxes.contains(r.commercial_taxes_profits));
            assert_eq!(o.income_taxes, r.income_taxes_profits);
            assert!(o.net().contains(r.net()), "{:?} {}", o.net(), r.net());
        }
    }
}
