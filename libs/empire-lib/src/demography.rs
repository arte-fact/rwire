use std::cmp::max;

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
    /// Nobles who left the court to flee a short ration.
    pub nobles_departed: i32,
    pub disease_victims: i32,
    pub malnutrition_victims: i32,
    pub starvation_victims: i32,
    /// The army's fighting strength for the coming year, in per cent (50–150).
    pub soldiers_efficiency: i32,
    pub soldiers_starvation_victims: i32,
    pub soldiers_desertion_victims: i32,
}

/// An expected figure with the spread of the random draw around it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Outlook {
    pub low: i32,
    pub expected: i32,
    pub high: i32,
}

// The curves. `r` is the ration: grain given over grain needed. The original
// BASIC drew every figure uniformly from 1 to a linear cap, so feeding the
// people just over half their needs still grew the population (births beat the
// extra deaths); these keep the births falling with the ration and make the
// hunger toll convex, so a short ration costs a little and half a ration is a
// famine.

/// Births: 5.3 % of the population a year, falling with the ration.
const BIRTH_RATE: f32 = 0.053;
const BIRTH_EXP: f32 = 0.7;
const BIRTH_SPREAD: f32 = 0.8;
/// Hunger deaths: 80 % of the population with no grain at all, convex.
const HUNGER_RATE: f32 = 0.8;
const HUNGER_EXP: f32 = 1.6;
const HUNGER_SPREAD: f32 = 0.2;
/// Below half a ration the extra toll is counted as starvation ("misère").
const STARVATION_RATION: f32 = 0.5;
/// Immigration: an S-curve of the ration — nobody comes on a plain ration, the
/// flow rises around one and a half and levels off near two, at 6 % of the
/// population a year before the immigration tax takes its share.
const IMMIGRATION_RATE: f32 = 0.06;
const IMMIGRATION_MID: f32 = 1.5;
const IMMIGRATION_WIDTH: f32 = 0.12;
const IMMIGRATION_SPREAD: f32 = 0.6;
/// One immigrant in fifty is a noble, one in two hundred and fifty a merchant.
const NOBLE_SHARE: f32 = 0.02;
const MERCHANT_SHARE: f32 = 0.004;
/// A few merchants settle every year regardless: 1 to 6 of them.
const MERCHANT_TRICKLE: (i32, i32) = (1, 7);
/// Nobles flee a short ration: 70 % of the court leaves with no grain at all.
const NOBLE_FLIGHT_RATE: f32 = 0.7;
const NOBLE_FLIGHT_EXP: f32 = 1.3;
const NOBLE_FLIGHT_SPREAD: f32 = 0.4;
/// Army losses: 90 % of the men with no grain at all.
const ARMY_LOSS_RATE: f32 = 0.9;
const ARMY_LOSS_EXP: f32 = 1.5;
/// The draw's spread around the expected losses, ±30 %.
pub const ARMY_LOSS_SPREAD: f32 = 0.3;

fn ration(given: i32, needs: i32) -> f32 {
    if needs <= 0 {
        1.0
    } else {
        given.max(0) as f32 / needs as f32
    }
}

fn births_expected(pop: i32, r: f32) -> f32 {
    pop as f32 * BIRTH_RATE * r.min(1.0).powf(BIRTH_EXP)
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

/// The mean of the yearly immigration: the S-curve of the ration, scaled by
/// the population and cut by the immigration tax (none at the 50 % cap).
fn immigrants_expected(kingdom: &Kingdom, r: f32) -> f32 {
    let pull = |r: f32| 1.0 / (1.0 + (-(r - IMMIGRATION_MID) / IMMIGRATION_WIDTH).exp());
    // Rebased so a plain ration draws exactly nobody.
    let floor = pull(1.0);
    let s = ((pull(r) - floor) / (1.0 - floor)).max(0.0);
    let tax = (1.0 - kingdom.immigration_taxes as f32 / 50.0).clamp(0.0, 1.0);
    kingdom.population() as f32 * IMMIGRATION_RATE * s * tax
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

/// A draw around `expected`, uniform within ±`spread` of it.
fn around(expected: f32, spread: f32) -> i32 {
    let lo = (expected * (1.0 - spread)).round() as i32;
    let hi = (expected * (1.0 + spread)).round() as i32;
    random(lo, hi + 1)
}

/// Original: DD=INT(RND*PO/22+1) where PO = serfs + merchants + nobles
fn disease_victims(kingdom: &Kingdom) -> i32 {
    random(1, kingdom.population() / 22 + 1)
}

pub fn demography_report(
    kingdom: &Kingdom,
    grain_for_peasants: i32,
    grain_for_soldiers: i32,
) -> YearDemography {
    let pop = kingdom.population();
    let r = ration(grain_for_peasants, kingdom.peasants_grain_needs());
    let (malnutrition, starvation) = hunger_split(pop, r);
    let ra = ration(grain_for_soldiers, kingdom.soldiers_grain_needs());
    let (desertion, army_starvation) = army_losses_split(kingdom.soldiers, ra);
    let immigrants = around(immigrants_expected(kingdom, r), IMMIGRATION_SPREAD);
    let nobles_immigrants = random(0, (immigrants as f32 * NOBLE_SHARE * 2.0) as i32 + 1);
    let merchants_immigrants = random(0, (immigrants as f32 * MERCHANT_SHARE * 2.0) as i32 + 1);

    YearDemography {
        births: around(births_expected(pop, r), BIRTH_SPREAD),
        immigrants,
        nobles_immigrants,
        merchants_immigrants,
        merchants_settled: random(MERCHANT_TRICKLE.0, MERCHANT_TRICKLE.1),
        nobles_departed: around(
            nobles_flight_expected(kingdom.nobles, r),
            NOBLE_FLIGHT_SPREAD,
        )
        .min(kingdom.nobles),
        disease_victims: disease_victims(kingdom),
        malnutrition_victims: around(malnutrition, HUNGER_SPREAD),
        starvation_victims: around(starvation, HUNGER_SPREAD),
        soldiers_efficiency: army_efficiency(kingdom, grain_for_soldiers),
        soldiers_starvation_victims: around(army_starvation, ARMY_LOSS_SPREAD),
        soldiers_desertion_victims: around(desertion, ARMY_LOSS_SPREAD),
    }
}

/// The mean and half-width of the merchants' yearly trickle.
fn merchant_trickle() -> (f32, f32) {
    let (lo, hi) = (MERCHANT_TRICKLE.0 as f32, (MERCHANT_TRICKLE.1 - 1) as f32);
    ((lo + hi) / 2.0, (hi - lo) / 2.0)
}

fn outlook(expected: f32, spread: f32) -> Outlook {
    Outlook {
        low: (expected - spread).round() as i32,
        expected: expected.round() as i32,
        high: (expected + spread).round() as i32,
    }
}

/// What feeding the people `grain` bushels should do to the headcount over the
/// year (births, immigrants and the merchants' trickle, less disease, hunger
/// and the nobles who leave), with the spread of the draws around it.
pub fn people_outlook(kingdom: &Kingdom, grain: i32) -> Outlook {
    let pop = kingdom.population();
    let r = ration(grain, kingdom.peasants_grain_needs());
    let births = births_expected(pop, r);
    let hunger = hunger_expected(pop, r);
    let disease = pop as f32 / 44.0;
    let immigrants = immigrants_expected(kingdom, r);
    let flight = nobles_flight_expected(kingdom.nobles, r);
    let (trickle, trickle_spread) = merchant_trickle();
    let expected = births + immigrants + trickle - disease - hunger - flight;
    // Each draw's full range, combined in quadrature.
    let spread = ((births * BIRTH_SPREAD).powi(2)
        + disease.powi(2)
        + (hunger * HUNGER_SPREAD).powi(2)
        + (immigrants * IMMIGRATION_SPREAD).powi(2)
        + (flight * NOBLE_FLIGHT_SPREAD).powi(2)
        + trickle_spread.powi(2))
    .sqrt();
    outlook(expected, spread)
}

/// What feeding the people `grain` bushels should do to the court: the nobles
/// among the immigrants, less those who flee a short ration.
pub fn nobles_outlook(kingdom: &Kingdom, grain: i32) -> Outlook {
    let r = ration(grain, kingdom.peasants_grain_needs());
    let arrivals = immigrants_expected(kingdom, r) * NOBLE_SHARE;
    let flight = nobles_flight_expected(kingdom.nobles, r);
    let spread = (arrivals.powi(2) + (flight * NOBLE_FLIGHT_SPREAD).powi(2)).sqrt();
    let o = outlook(arrivals - flight, spread);
    Outlook {
        low: o.low.max(-kingdom.nobles),
        expected: o.expected.max(-kingdom.nobles),
        high: o.high.max(-kingdom.nobles),
    }
}

/// The share of the army expected to be lost for `grain` bushels (0–0.9), the
/// smooth curve behind [`army_outlook`]'s rounded figures.
pub fn army_losses_share(kingdom: &Kingdom, grain: i32) -> f32 {
    let r = ration(grain, kingdom.soldiers_grain_needs());
    shortfall(1, r, ARMY_LOSS_RATE, ARMY_LOSS_EXP)
}

/// The army's efficiency (50–150 %) and its expected losses for `grain` bushels.
pub fn army_outlook(kingdom: &Kingdom, grain: i32) -> (i32, Outlook) {
    let r = ration(grain, kingdom.soldiers_grain_needs());
    let losses = army_losses_expected(kingdom.soldiers, r);
    // The draws may overshoot the headcount at the bottom; the army can't lose
    // more men than it has.
    let bounded = |x: f32| (x.round() as i32).min(kingdom.soldiers);
    let outlook = Outlook {
        low: bounded(losses * (1.0 - ARMY_LOSS_SPREAD)),
        expected: bounded(losses),
        high: bounded(losses * (1.0 + ARMY_LOSS_SPREAD)),
    };
    (army_efficiency(kingdom, grain), outlook)
}

/// Feed the population and the army for the year: consumes both grain
/// allocations and applies births, deaths, immigration and army morale.
pub fn apply_feed(
    kingdom: &mut Kingdom,
    grain_for_peasants: i32,
    grain_for_soldiers: i32,
) -> YearDemography {
    let report = demography_report(kingdom, grain_for_peasants, grain_for_soldiers);

    kingdom.grain_stocks -= grain_for_peasants + grain_for_soldiers;

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
    kingdom.merchants += report.merchants_immigrants + report.merchants_settled;

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Kingdoms;

    #[test]
    fn well_fed_population_has_no_hunger_deaths() {
        let k = Kingdom::new(Kingdoms::France);
        let r = demography_report(&k, k.peasants_grain_needs(), k.soldiers_grain_needs());
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
        let r = demography_report(&k, 0, k.soldiers_grain_needs() * 3 / 4);
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
        let r = demography_report(&k, needs / 2, k.soldiers_grain_needs() / 2);
        assert_eq!(r.starvation_victims, 0);
        assert!(r.malnutrition_victims > 0);
        assert_eq!(r.soldiers_starvation_victims, 0);
        assert!(r.soldiers_desertion_victims > 0);
        let r = demography_report(&k, needs / 4, 0);
        assert!(r.starvation_victims > 0);
        assert!(r.soldiers_starvation_victims > 0);
    }

    #[test]
    fn efficiency_is_clamped_and_handles_empty_army() {
        let mut k = Kingdom::new(Kingdoms::France);
        assert_eq!(demography_report(&k, 0, 1_000_000).soldiers_efficiency, 150);
        assert_eq!(demography_report(&k, 0, 0).soldiers_efficiency, 50);
        k.soldiers = 0;
        assert_eq!(demography_report(&k, 0, 10).soldiers_efficiency, 150);
        assert_eq!(demography_report(&k, 0, 0).soldiers_efficiency, 100);
    }

    #[test]
    fn efficiency_is_logarithmic() {
        let k = Kingdom::new(Kingdoms::France);
        let needs = k.soldiers_grain_needs();
        assert_eq!(army_outlook(&k, needs * 3 / 2).0, 150);
        assert_eq!(army_outlook(&k, needs * 5 / 4).0, 128);
        assert_eq!(army_outlook(&k, needs * 3 / 4).0, 65);
        assert_eq!(army_outlook(&k, needs * 2 / 3).0, 50);
        assert_eq!(army_outlook(&k, needs / 2).0, 50);
    }

    #[test]
    fn short_rations_shrink_the_population() {
        let k = Kingdom::new(Kingdoms::France);
        let needs = k.peasants_grain_needs();
        assert!(people_outlook(&k, needs).expected > 0);
        assert!(people_outlook(&k, needs * 85 / 100).expected < 0);
        // Feeding just over half the needs used to grow the population.
        let half = people_outlook(&k, needs * 51 / 100);
        assert!(half.high < 0);
        assert!(half.expected < -(k.population() / 5));
        let none = people_outlook(&k, 0);
        assert!(none.expected < -(k.population() * 3 / 4));
        assert!(none.low >= -k.population());
    }

    #[test]
    fn immigration_follows_an_s_curve() {
        let k = Kingdom::new(Kingdoms::France);
        let at = |pct: i32| immigrants_expected(&k, pct as f32 / 100.0);
        assert_eq!(at(100), 0.0);
        assert_eq!(at(80), 0.0);
        assert!(at(120) > 0.0 && at(120) < at(150) / 5.0);
        let mid = k.population() as f32 * IMMIGRATION_RATE * 0.5 * 0.6;
        assert!((at(150) - mid).abs() < 2.0, "{}", at(150));
        assert!(at(200) > at(190) && at(200) - at(190) < (at(160) - at(150)) / 5.0);
        assert!(at(200) < k.population() as f32 * IMMIGRATION_RATE);
        let mut taxed = k.clone();
        taxed.immigration_taxes = 50;
        assert_eq!(immigrants_expected(&taxed, 2.0), 0.0);
        taxed.immigration_taxes = 0;
        assert!(immigrants_expected(&taxed, 2.0) > at(200));
    }

    #[test]
    fn nobles_come_with_plenty_and_flee_the_famine() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 10;
        let needs = k.peasants_grain_needs();
        assert_eq!(nobles_outlook(&k, needs), Outlook::default());
        assert!(nobles_outlook(&k, needs * 2).expected > 0);
        let famine = nobles_outlook(&k, 0);
        assert!(famine.expected <= -6 && famine.low >= -10);
        assert!(nobles_outlook(&k, needs * 3 / 4).expected < 0);
    }

    #[test]
    fn army_losses_never_exceed_the_army() {
        let k = Kingdom::new(Kingdoms::France);
        let (efficiency, losses) = army_outlook(&k, 0);
        assert_eq!(efficiency, 50);
        assert!(losses.high <= k.soldiers);
        assert!(losses.low > 0);
    }

    #[test]
    fn outlooks_grow_with_the_ration() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 10;
        let needs = k.peasants_grain_needs();
        let mut last = people_outlook(&k, 0);
        let mut last_nobles = nobles_outlook(&k, 0);
        for pct in 1..=200 {
            let o = people_outlook(&k, needs * pct / 100);
            assert!(o.expected >= last.expected, "{pct} %: {o:?} < {last:?}");
            assert!(o.low <= o.expected && o.expected <= o.high);
            last = o;
            let n = nobles_outlook(&k, needs * pct / 100);
            assert!(n.expected >= last_nobles.expected);
            last_nobles = n;
        }
        let army = k.soldiers_grain_needs();
        let mut last = army_outlook(&k, 0);
        for pct in 1..=150 {
            let o = army_outlook(&k, army * pct / 100);
            assert!(o.0 >= last.0 && o.1.expected <= last.1.expected);
            last = o;
        }
        assert_eq!(army_outlook(&k, army).1, Outlook::default());
    }

    #[test]
    fn apply_feed_consumes_grain_once_and_seats_the_newcomers() {
        let mut k = Kingdom::new(Kingdoms::France);
        let before = k.clone();
        let r = apply_feed(&mut k, 1000, 200);
        assert_eq!(k.grain_stocks, before.grain_stocks - 1200);
        assert!(k.peasants >= 0 && k.soldiers >= 0);
        assert_eq!(
            k.nobles,
            (before.nobles + r.nobles_immigrants - r.nobles_departed).max(0)
        );
        assert_eq!(
            k.merchants,
            before.merchants + r.merchants_immigrants + r.merchants_settled
        );
    }
}
