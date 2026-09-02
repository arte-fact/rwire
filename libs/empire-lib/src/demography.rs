use std::cmp::max;
use std::ops::{Add, Sub};

use crate::economy::Taxes;
use crate::kingdom::Kingdom;
use crate::random::random;

#[derive(Debug, Clone)]
pub struct YearDemography {
    pub births: i32,
    /// Foreigners settled this year, nobles and merchants among them included.
    pub immigrants: i32,
    /// The nobles and merchants among the immigrants.
    pub nobles_immigrants: i32,
    pub merchants_immigrants: i32,
    /// The few merchants who set up shop every year regardless.
    pub merchants_settled: i32,
    /// Nobles who left the court to flee a short ration or a heavy taille.
    pub nobles_departed: i32,
    /// Merchants who closed shop under the sales tax.
    pub merchants_departed: i32,
    pub disease_victims: i32,
    pub malnutrition_victims: i32,
    pub starvation_victims: i32,
    /// The army's fighting strength for the coming year, in per cent (50–150).
    pub soldiers_efficiency: i32,
    pub soldiers_starvation_victims: i32,
    pub soldiers_desertion_victims: i32,
}

impl YearDemography {
    /// Net change of the civilian headcount over the year.
    pub fn population_delta(&self) -> i32 {
        self.births + self.immigrants + self.merchants_settled
            - self.disease_victims
            - self.nobles_departed
            - self.merchants_departed
            - self.malnutrition_victims
            - self.starvation_victims
    }
}

/// Hard bounds on a figure: the draw lands between `low` and `high`, never
/// outside. `expected` is the middle of the draw — a marker, not a promise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Outlook {
    pub low: i32,
    pub expected: i32,
    pub high: i32,
}

impl Outlook {
    pub fn sure(v: i32) -> Self {
        Outlook {
            low: v,
            expected: v,
            high: v,
        }
    }

    pub fn plus(self, v: i32) -> Outlook {
        self + Outlook::sure(v)
    }

    pub fn minus(self, v: i32) -> Outlook {
        self - Outlook::sure(v)
    }

    pub fn at_most(self, cap: i32) -> Outlook {
        Outlook {
            low: self.low.min(cap),
            expected: self.expected.min(cap),
            high: self.high.min(cap),
        }
    }

    pub fn at_least(self, floor: i32) -> Outlook {
        Outlook {
            low: self.low.max(floor),
            expected: self.expected.max(floor),
            high: self.high.max(floor),
        }
    }

    pub fn contains(self, v: i32) -> bool {
        (self.low..=self.high).contains(&v)
    }

    /// A draw uniform within `±spread` of `expected` (the bounds rounded).
    fn around(expected: f32, spread: f32) -> Outlook {
        Outlook {
            low: (expected * (1.0 - spread)).round() as i32,
            expected: expected.round() as i32,
            high: (expected * (1.0 + spread)).round() as i32,
        }
    }

    /// A draw uniform from 0 to `cap` included.
    fn up_to(cap: i32, expected: f32) -> Outlook {
        Outlook {
            low: 0,
            expected: expected.round() as i32,
            high: cap.max(0),
        }
    }

    /// A uniform draw inside the bounds.
    fn draw(self) -> i32 {
        random(self.low, self.high + 1)
    }
}

/// Bounds of a sum: the extremes add up.
impl Add for Outlook {
    type Output = Outlook;
    fn add(self, o: Outlook) -> Outlook {
        Outlook {
            low: self.low + o.low,
            expected: self.expected + o.expected,
            high: self.high + o.high,
        }
    }
}

/// Bounds of a difference: the worst of one against the best of the other.
impl Sub for Outlook {
    type Output = Outlook;
    fn sub(self, o: Outlook) -> Outlook {
        Outlook {
            low: self.low - o.high,
            expected: self.expected - o.expected,
            high: self.high - o.low,
        }
    }
}

/// The council's decision for the year: the two grain allocations and the
/// three tax rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Council {
    pub grain_for_peasants: i32,
    pub grain_for_soldiers: i32,
    pub taxes: Taxes,
}

// The curves. `r` is the ration: grain given over grain needed. The original
// BASIC drew every figure uniformly from 1 to a linear cap, so feeding the
// people just over half their needs still grew the population (births beat the
// extra deaths); these keep the births falling with the ration and make the
// hunger toll convex, so a short ration costs a little and half a ration is a
// famine. `x`, `y`, `z` are the customs, sales and income tax rates over their
// caps (see [`Taxes`]).

/// Births: 5.3 % of the population a year, falling with the ration, and a
/// third fewer under the heaviest taille.
const BIRTH_RATE: f32 = 0.053;
const BIRTH_EXP: f32 = 0.7;
const BIRTH_TAILLE: f32 = 0.35;
const BIRTH_SPREAD: f32 = 0.8;
/// Hunger deaths: 80 % of the population with no grain at all, convex.
const HUNGER_RATE: f32 = 0.8;
const HUNGER_EXP: f32 = 1.6;
const HUNGER_SPREAD: f32 = 0.2;
/// Below half a ration the extra toll is counted as starvation ("misère").
const STARVATION_RATION: f32 = 0.5;
/// Immigration: an S-curve of the ration — nobody comes on a plain ration, the
/// flow rises around one and a half and levels off near two, at 6 % of the
/// population a year before the customs take their share (nobody at the cap).
const IMMIGRATION_RATE: f32 = 0.06;
const IMMIGRATION_MID: f32 = 1.5;
const IMMIGRATION_WIDTH: f32 = 0.12;
const IMMIGRATION_CUSTOMS_EXP: f32 = 1.6;
const IMMIGRATION_SPREAD: f32 = 0.6;
/// One immigrant in fifty is a noble, one in two hundred and fifty a merchant;
/// the nobles balk at the customs, the merchants at the sales tax.
const NOBLE_SHARE: f32 = 0.02;
const MERCHANT_SHARE: f32 = 0.004;
/// A few merchants settle every year regardless: 1 to 6 of them.
const MERCHANT_TRICKLE: (i32, i32) = (1, 6);
/// Merchants close shop under the sales tax: 8 % of them at the cap, cubic.
const MERCHANT_FLIGHT_RATE: f32 = 0.08;
/// Nobles flee a short ration: 70 % of the court leaves with no grain at all.
const NOBLE_FLIGHT_RATE: f32 = 0.7;
const NOBLE_FLIGHT_EXP: f32 = 1.3;
const NOBLE_FLIGHT_SPREAD: f32 = 0.4;
/// …and a heavy taille: a quarter of the court, from 25 % on (a sigmoid
/// centred at three quarters of the cap, nobody below 20 %).
const NOBLE_TAILLE_RATE: f32 = 0.25;
const NOBLE_TAILLE_MID: f32 = 0.75;
const NOBLE_TAILLE_WIDTH: f32 = 0.05;
/// Army losses: 90 % of the men with no grain at all.
const ARMY_LOSS_RATE: f32 = 0.9;
const ARMY_LOSS_EXP: f32 = 1.5;
/// The draw's spread around the expected losses, ±30 %.
const ARMY_LOSS_SPREAD: f32 = 0.3;

fn ration(given: i32, needs: i32) -> f32 {
    if needs <= 0 {
        1.0
    } else {
        given.max(0) as f32 / needs as f32
    }
}

fn births_expected(pop: i32, r: f32, z: f32) -> f32 {
    pop as f32 * BIRTH_RATE * r.min(1.0).powf(BIRTH_EXP) * (1.0 - BIRTH_TAILLE * z * z)
}

/// `rate × (1 − r)^exp` of `count` below a full ration, nothing above.
fn shortfall(count: i32, r: f32, rate: f32, exp: f32) -> f32 {
    if r < 1.0 {
        count as f32 * rate * (1.0 - r).powf(exp)
    } else {
        0.0
    }
}

fn hunger_expected(pop: i32, r: f32) -> f32 {
    shortfall(pop, r, HUNGER_RATE, HUNGER_EXP)
}

/// Hunger deaths split in two lines: the toll down to half a ration, then the
/// extra toll of a famine below it.
fn hunger_split(pop: i32, r: f32) -> (f32, f32) {
    let all = hunger_expected(pop, r);
    let malnutrition = hunger_expected(pop, r.max(STARVATION_RATION));
    (malnutrition, all - malnutrition)
}

fn army_losses_expected(soldiers: i32, r: f32) -> f32 {
    shortfall(soldiers, r, ARMY_LOSS_RATE, ARMY_LOSS_EXP)
}

fn army_losses_split(soldiers: i32, r: f32) -> (f32, f32) {
    let all = army_losses_expected(soldiers, r);
    let desertion = army_losses_expected(soldiers, r.max(STARVATION_RATION));
    (desertion, all - desertion)
}

fn nobles_flight_expected(nobles: i32, r: f32) -> f32 {
    shortfall(nobles, r, NOBLE_FLIGHT_RATE, NOBLE_FLIGHT_EXP)
}

/// The nobles who leave over the taille, sure.
fn nobles_taxed_away(nobles: i32, z: f32) -> i32 {
    let s = 1.0 / (1.0 + (-(z - NOBLE_TAILLE_MID) / NOBLE_TAILLE_WIDTH).exp());
    (nobles as f32 * NOBLE_TAILLE_RATE * s).round() as i32
}

/// The merchants who close shop over the sales tax, sure.
fn merchants_departed(merchants: i32, y: f32) -> i32 {
    (merchants as f32 * MERCHANT_FLIGHT_RATE * y * y * y).round() as i32
}

/// The mean of the yearly immigration: the S-curve of the ration, scaled by
/// the population and cut by the customs (none at the cap).
fn immigrants_expected(pop: i32, r: f32, x: f32) -> f32 {
    let pull = |r: f32| 1.0 / (1.0 + (-(r - IMMIGRATION_MID) / IMMIGRATION_WIDTH).exp());
    // Rebased so a plain ration draws exactly nobody.
    let floor = pull(1.0);
    let s = ((pull(r) - floor) / (1.0 - floor)).max(0.0);
    pop as f32 * IMMIGRATION_RATE * s * (1.0 - x.powf(IMMIGRATION_CUSTOMS_EXP)).max(0.0)
}

/// Original: nobles among the immigrants drawn from 0 to twice their share.
fn nobles_among(immigrants: i32, x: f32) -> i32 {
    (immigrants as f32 * NOBLE_SHARE * 2.0 * (1.0 - x * x)) as i32
}

fn merchants_among(immigrants: i32, y: f32) -> i32 {
    (immigrants as f32 * MERCHANT_SHARE * 2.0 * (1.0 - y * y)) as i32
}

/// Efficiency in per cent (50–150): logarithmic in the ration, so extra
/// rations pay less and less while short rations cost fast — 150 % at one and
/// a half rations, 50 % at two thirds of one (¾ of a ration → 65 %).
fn army_efficiency(kingdom: &Kingdom, grain_for_soldiers: i32) -> i32 {
    // No soldiers but grain given → max efficiency (preparing a future army);
    // no soldiers and no grain → default 100 %.
    if kingdom.soldiers == 0 {
        return if grain_for_soldiers > 0 { 150 } else { 100 };
    }
    let r = ration(grain_for_soldiers, kingdom.soldiers_grain_needs());
    if r <= 0.0 {
        return 50;
    }
    ((100.0 + 50.0 * r.ln() / 1.5f32.ln()).round() as i32).clamp(50, 150)
}

/// Every draw of the year as bounds. The report draws inside each; the
/// forecast adds the bounds up — so the forecast brackets the report.
struct Draws {
    births: Outlook,
    disease: Outlook,
    malnutrition: Outlook,
    starvation: Outlook,
    immigrants: Outlook,
    nobles_immigrants: Outlook,
    merchants_immigrants: Outlook,
    merchants_settled: Outlook,
    nobles_departed: Outlook,
    merchants_departed: i32,
    soldiers_efficiency: i32,
    desertion: Outlook,
    army_starvation: Outlook,
}

fn draws(kingdom: &Kingdom, council: Council) -> Draws {
    let pop = kingdom.population();
    let (x, y, z) = council.taxes.shares();
    let r = ration(council.grain_for_peasants, kingdom.peasants_grain_needs());
    let (malnutrition, starvation) = hunger_split(pop, r);
    let ra = ration(council.grain_for_soldiers, kingdom.soldiers_grain_needs());
    let (desertion, army_starvation) = army_losses_split(kingdom.soldiers, ra);
    let immigrants = Outlook::around(immigrants_expected(pop, r, x), IMMIGRATION_SPREAD);
    // Original: DD=INT(RND*PO/22+1) where PO = serfs + merchants + nobles
    let disease_cap = pop / 22;
    let disease = Outlook {
        low: disease_cap.min(1),
        expected: (disease_cap.max(1) + 1) / 2,
        high: disease_cap,
    };
    let trickle = 1.0 - y * y;
    let flight = Outlook::around(
        nobles_flight_expected(kingdom.nobles, r),
        NOBLE_FLIGHT_SPREAD,
    );
    Draws {
        births: Outlook::around(births_expected(pop, r, z), BIRTH_SPREAD),
        disease,
        malnutrition: Outlook::around(malnutrition, HUNGER_SPREAD),
        starvation: Outlook::around(starvation, HUNGER_SPREAD),
        immigrants,
        nobles_immigrants: Outlook::up_to(
            nobles_among(immigrants.high, x),
            immigrants.expected as f32 * NOBLE_SHARE * (1.0 - x * x),
        ),
        merchants_immigrants: Outlook::up_to(
            merchants_among(immigrants.high, y),
            immigrants.expected as f32 * MERCHANT_SHARE * (1.0 - y * y),
        ),
        merchants_settled: Outlook {
            low: (MERCHANT_TRICKLE.0 as f32 * trickle).round() as i32,
            expected: ((MERCHANT_TRICKLE.0 + MERCHANT_TRICKLE.1) as f32 / 2.0 * trickle).round()
                as i32,
            high: (MERCHANT_TRICKLE.1 as f32 * trickle).round() as i32,
        },
        nobles_departed: flight
            .plus(nobles_taxed_away(kingdom.nobles, z))
            .at_most(kingdom.nobles),
        merchants_departed: merchants_departed(kingdom.merchants, y),
        soldiers_efficiency: army_efficiency(kingdom, council.grain_for_soldiers),
        desertion: Outlook::around(desertion, ARMY_LOSS_SPREAD),
        army_starvation: Outlook::around(army_starvation, ARMY_LOSS_SPREAD),
    }
}

pub fn demography_report(kingdom: &Kingdom, council: Council) -> YearDemography {
    let d = draws(kingdom, council);
    let (x, y, _) = council.taxes.shares();
    let immigrants = d.immigrants.draw();
    let trickle = 1.0 - y * y;
    let nobles_departed = d.nobles_departed.draw();
    // The draws may overshoot the headcount at the bottom: nobody dies twice.
    let disease_victims = d.disease.draw();
    let malnutrition_victims = d.malnutrition.draw();
    let left = kingdom.population() - nobles_departed - d.merchants_departed;
    let starvation_victims = d
        .starvation
        .draw()
        .min(left - disease_victims - malnutrition_victims)
        .max(0);
    let soldiers_desertion_victims = d.desertion.draw();
    let soldiers_starvation_victims = d
        .army_starvation
        .draw()
        .min(kingdom.soldiers - soldiers_desertion_victims)
        .max(0);
    YearDemography {
        births: d.births.draw(),
        immigrants,
        nobles_immigrants: random(0, nobles_among(immigrants, x) + 1),
        merchants_immigrants: random(0, merchants_among(immigrants, y) + 1),
        merchants_settled: (random(MERCHANT_TRICKLE.0, MERCHANT_TRICKLE.1 + 1) as f32 * trickle)
            .round() as i32,
        nobles_departed,
        merchants_departed: d.merchants_departed,
        disease_victims,
        malnutrition_victims,
        starvation_victims,
        soldiers_efficiency: d.soldiers_efficiency,
        soldiers_starvation_victims,
        soldiers_desertion_victims,
    }
}

/// What the council's decision should do to the headcounts over the year,
/// as hard bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemographyOutlook {
    /// Civilians: births, immigrants and the merchants' trickle, less disease,
    /// hunger and those who leave.
    pub people: Outlook,
    pub births: Outlook,
    /// The court: the nobles among the immigrants, less those who leave.
    pub nobles: Outlook,
    pub nobles_immigrants: Outlook,
    pub nobles_departed: Outlook,
    pub merchants: Outlook,
    pub immigrants: Outlook,
    /// The army's efficiency (50–150 %), sure.
    pub soldiers_efficiency: i32,
    pub army_losses: Outlook,
}

pub fn demography_outlook(kingdom: &Kingdom, council: Council) -> DemographyOutlook {
    let d = draws(kingdom, council);
    DemographyOutlook {
        people: (d.births + d.immigrants + d.merchants_settled
            - d.disease
            - d.malnutrition
            - d.starvation
            - d.nobles_departed)
            .minus(d.merchants_departed)
            .at_least(-kingdom.population()),
        births: d.births,
        nobles: d.nobles_immigrants - d.nobles_departed,
        nobles_immigrants: d.nobles_immigrants,
        nobles_departed: d.nobles_departed,
        merchants: (d.merchants_immigrants + d.merchants_settled).minus(d.merchants_departed),
        immigrants: d.immigrants,
        soldiers_efficiency: d.soldiers_efficiency,
        // The draws may overshoot the headcount at the bottom; the army can't
        // lose more men than it has.
        army_losses: (d.desertion + d.army_starvation).at_most(kingdom.soldiers),
    }
}

/// The share of the army expected to be lost for `grain` bushels (0–0.9), the
/// smooth curve behind the outlook's rounded figures.
pub fn army_losses_share(kingdom: &Kingdom, grain: i32) -> (f32, f32) {
    let r = ration(grain, kingdom.soldiers_grain_needs());
    let share = shortfall(1, r, ARMY_LOSS_RATE, ARMY_LOSS_EXP);
    (share, ARMY_LOSS_SPREAD)
}

/// Feed the population and the army for the year: consumes both grain
/// allocations and applies births, deaths, immigration and army morale.
pub fn apply_feed(kingdom: &mut Kingdom, council: Council) -> YearDemography {
    let report = demography_report(kingdom, council);

    kingdom.grain_stocks -= council.grain_for_peasants + council.grain_for_soldiers;

    // The nobles and merchants among the immigrants join their own ranks.
    let peasant_change = report.births + report.immigrants
        - report.nobles_immigrants
        - report.merchants_immigrants
        - report.starvation_victims
        - report.malnutrition_victims
        - report.disease_victims;
    kingdom.peasants = max(0, kingdom.peasants + peasant_change);

    let soldier_losses = report.soldiers_starvation_victims + report.soldiers_desertion_victims;
    kingdom.soldiers = max(0, kingdom.soldiers - soldier_losses);
    kingdom.soldiers_efficiency = report.soldiers_efficiency;

    kingdom.nobles = max(
        0,
        kingdom.nobles + report.nobles_immigrants - report.nobles_departed,
    );
    kingdom.merchants = max(
        0,
        kingdom.merchants + report.merchants_immigrants + report.merchants_settled
            - report.merchants_departed,
    );

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Kingdoms;

    fn council(k: &Kingdom, peasants: i32, soldiers: i32) -> Council {
        Council {
            grain_for_peasants: peasants,
            grain_for_soldiers: soldiers,
            taxes: k.taxes(),
        }
    }

    fn taxed(k: &Kingdom, peasants: i32, customs: i32, sales: i32, income: i32) -> Council {
        Council {
            grain_for_peasants: peasants,
            grain_for_soldiers: k.soldiers_grain_needs(),
            taxes: Taxes {
                customs,
                sales,
                income,
            },
        }
    }

    #[test]
    fn well_fed_population_has_no_hunger_deaths() {
        let k = Kingdom::new(Kingdoms::France);
        let r = demography_report(
            &k,
            council(&k, k.peasants_grain_needs(), k.soldiers_grain_needs()),
        );
        assert_eq!(r.malnutrition_victims, 0);
        assert_eq!(r.starvation_victims, 0);
        assert_eq!(r.soldiers_starvation_victims, 0);
        assert_eq!(r.soldiers_desertion_victims, 0);
        assert_eq!(r.nobles_departed, 0);
        assert_eq!(r.immigrants, 0);
        assert_eq!(r.soldiers_efficiency, 100);
        assert!(r.births > 0);
        assert!((1..=6).contains(&r.merchants_settled));
    }

    #[test]
    fn starving_population_dies_and_army_deserts() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 20;
        let r = demography_report(&k, council(&k, 0, k.soldiers_grain_needs() * 3 / 4));
        assert!(r.starvation_victims > 0);
        assert!(r.malnutrition_victims > 0);
        assert_eq!(r.births, 0);
        assert!(r.nobles_departed > 0 && r.nobles_departed <= 20);
        assert!(r.soldiers_desertion_victims > 0);
        assert_eq!(r.soldiers_starvation_victims, 0);
        assert_eq!(r.soldiers_efficiency, 65);
    }

    #[test]
    fn starvation_only_below_half_a_ration() {
        let k = Kingdom::new(Kingdoms::France);
        let needs = k.peasants_grain_needs();
        let r = demography_report(&k, council(&k, needs / 2, k.soldiers_grain_needs() / 2));
        assert_eq!(r.starvation_victims, 0);
        assert!(r.malnutrition_victims > 0);
        assert_eq!(r.soldiers_starvation_victims, 0);
        assert!(r.soldiers_desertion_victims > 0);
        let r = demography_report(&k, council(&k, needs / 4, 0));
        assert!(r.starvation_victims > 0);
        assert!(r.soldiers_starvation_victims > 0);
    }

    #[test]
    fn efficiency_is_clamped_and_handles_empty_army() {
        let mut k = Kingdom::new(Kingdoms::France);
        let eff = |k: &Kingdom, g: i32| demography_report(k, council(k, 0, g)).soldiers_efficiency;
        assert_eq!(eff(&k, 1_000_000), 150);
        assert_eq!(eff(&k, 0), 50);
        k.soldiers = 0;
        assert_eq!(eff(&k, 10), 150);
        assert_eq!(eff(&k, 0), 100);
    }

    #[test]
    fn efficiency_is_logarithmic() {
        let k = Kingdom::new(Kingdoms::France);
        let needs = k.soldiers_grain_needs();
        let eff = |g: i32| demography_outlook(&k, council(&k, 0, g)).soldiers_efficiency;
        assert_eq!(eff(needs * 3 / 2), 150);
        assert_eq!(eff(needs * 5 / 4), 128);
        assert_eq!(eff(needs * 3 / 4), 65);
        assert_eq!(eff(needs * 2 / 3), 50);
        assert_eq!(eff(needs / 2), 50);
    }

    #[test]
    fn short_rations_shrink_the_population() {
        let k = Kingdom::new(Kingdoms::France);
        let needs = k.peasants_grain_needs();
        let people = |g: i32| demography_outlook(&k, council(&k, g, 0)).people;
        assert!(people(needs).expected > 0);
        assert!(people(needs * 85 / 100).expected < 0);
        // Feeding just over half the needs used to grow the population.
        let half = people(needs * 51 / 100);
        assert!(half.high < 0);
        assert!(half.expected < -(k.population() / 5));
        let none = people(0);
        assert!(none.expected < -(k.population() * 3 / 4));
        assert!(none.low >= -k.population());
    }

    #[test]
    fn immigration_follows_an_s_curve() {
        let k = Kingdom::new(Kingdoms::France);
        let pop = k.population();
        let at = |pct: i32| immigrants_expected(pop, pct as f32 / 100.0, 0.4);
        assert_eq!(at(100), 0.0);
        assert_eq!(at(80), 0.0);
        assert!(at(120) > 0.0 && at(120) < at(150) / 5.0);
        let cut = 1.0 - 0.4f32.powf(IMMIGRATION_CUSTOMS_EXP);
        let mid = pop as f32 * IMMIGRATION_RATE * 0.5 * cut;
        assert!((at(150) - mid).abs() < 2.0, "{}", at(150));
        assert!(at(200) > at(190) && at(200) - at(190) < (at(160) - at(150)) / 5.0);
        assert!(at(200) < pop as f32 * IMMIGRATION_RATE);
        assert_eq!(immigrants_expected(pop, 2.0, 1.0), 0.0);
        assert!(immigrants_expected(pop, 2.0, 0.0) > at(200));
    }

    #[test]
    fn nobles_come_with_plenty_and_flee_the_famine() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 10;
        let needs = k.peasants_grain_needs();
        let nobles = |g: i32| demography_outlook(&k, council(&k, g, 0)).nobles;
        assert_eq!(nobles(needs), Outlook::default());
        assert!(nobles(needs * 2).expected > 0);
        let famine = nobles(0);
        assert!(famine.expected <= -6 && famine.low >= -10);
        assert!(nobles(needs * 3 / 4).expected < 0);
    }

    #[test]
    fn taxes_weigh_on_the_people() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 20;
        k.merchants = 50;
        let needs = k.peasants_grain_needs();
        let at = |c, s, i| demography_outlook(&k, taxed(&k, needs * 3 / 2, c, s, i));
        let mild = at(20, 8, 20);
        // Customs cut the immigration, to nothing at the cap.
        assert!(at(0, 8, 20).immigrants.expected > mild.immigrants.expected);
        assert_eq!(at(50, 8, 20).immigrants, Outlook::default());
        // The taille cuts the births and, past 25 %, empties the court.
        assert!(at(20, 8, 35).people.expected < mild.people.expected);
        assert_eq!(at(20, 8, 20).nobles.low, 0);
        assert!(at(20, 8, 30).nobles.high < 0);
        assert!(at(20, 8, 35).nobles.low >= -20);
        // The sales tax closes shops.
        assert!(mild.merchants.low > 0);
        assert!(at(20, 20, 20).merchants.high < 0);
    }

    #[test]
    fn army_losses_never_exceed_the_army() {
        let k = Kingdom::new(Kingdoms::France);
        let o = demography_outlook(&k, council(&k, 0, 0));
        assert_eq!(o.soldiers_efficiency, 50);
        assert!(o.army_losses.high <= k.soldiers);
        assert!(o.army_losses.low > 0);
    }

    #[test]
    fn outlooks_grow_with_the_ration() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 10;
        let needs = k.peasants_grain_needs();
        let mut last = demography_outlook(&k, council(&k, 0, 0));
        for pct in 1..=200 {
            let o = demography_outlook(&k, council(&k, needs * pct / 100, 0));
            assert!(
                o.people.expected >= last.people.expected,
                "{pct} %: {o:?} < {last:?}"
            );
            assert!(o.people.low <= o.people.expected && o.people.expected <= o.people.high);
            assert!(o.nobles.expected >= last.nobles.expected);
            last = o;
        }
        let army = k.soldiers_grain_needs();
        let mut last = demography_outlook(&k, council(&k, 0, 0));
        for pct in 1..=150 {
            let o = demography_outlook(&k, council(&k, 0, army * pct / 100));
            assert!(o.soldiers_efficiency >= last.soldiers_efficiency);
            assert!(o.army_losses.expected <= last.army_losses.expected);
            last = o;
        }
        assert_eq!(last.army_losses, Outlook::default());
    }

    /// The forecast's contract: the report never lands outside its bounds.
    #[test]
    fn outlook_brackets_the_report() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 15;
        k.merchants = 60;
        k.peasants = 2400;
        k.soldiers = 80;
        let needs = k.peasants_grain_needs();
        let army = k.soldiers_grain_needs();
        let cases = [
            taxed(&k, needs * 3 / 2, 20, 8, 20),
            taxed(&k, needs * 2, 0, 0, 0),
            taxed(&k, needs * 7 / 10, 50, 20, 35),
            taxed(&k, needs / 3, 27, 15, 30),
            Council {
                grain_for_soldiers: army / 2,
                ..taxed(&k, needs, 20, 8, 20)
            },
            Council {
                grain_for_soldiers: 0,
                ..taxed(&k, 0, 10, 4, 10)
            },
        ];
        for c in cases {
            let o = demography_outlook(&k, c);
            for _ in 0..1000 {
                let r = demography_report(&k, c);
                assert!(
                    o.people.contains(r.population_delta()),
                    "{c:?}: {o:?} {r:?}"
                );
                assert!(
                    o.nobles.contains(r.nobles_immigrants - r.nobles_departed),
                    "{c:?}: {o:?} {r:?}"
                );
                assert!(
                    o.merchants.contains(
                        r.merchants_immigrants + r.merchants_settled - r.merchants_departed
                    ),
                    "{c:?}: {o:?} {r:?}"
                );
                assert!(o.immigrants.contains(r.immigrants));
                assert_eq!(o.soldiers_efficiency, r.soldiers_efficiency);
                assert!(o
                    .army_losses
                    .contains(r.soldiers_desertion_victims + r.soldiers_starvation_victims));
            }
        }
    }

    #[test]
    fn apply_feed_consumes_grain_once_and_seats_the_newcomers() {
        let mut k = Kingdom::new(Kingdoms::France);
        let before = k.clone();
        let r = apply_feed(&mut k, council(&before, 1000, 200));
        assert_eq!(k.grain_stocks, before.grain_stocks - 1200);
        assert!(k.peasants >= 0 && k.soldiers >= 0);
        assert_eq!(
            k.nobles,
            (before.nobles + r.nobles_immigrants - r.nobles_departed).max(0)
        );
        assert_eq!(
            k.merchants,
            before.merchants + r.merchants_immigrants + r.merchants_settled - r.merchants_departed
        );
    }
}
