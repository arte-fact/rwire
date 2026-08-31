use std::cmp::max;

use crate::kingdom::Kingdom;
use crate::random::random;

#[derive(Debug, Clone)]
pub struct YearDemography {
    pub births: i32,
    pub immigrants: i32,
    pub disease_victims: i32,
    pub malnutrition_victims: i32,
    pub starvation_victims: i32,
    pub soldiers_efficiency: i32,
    pub soldiers_starvation_victims: i32,
    pub soldiers_desertion_victims: i32,
}

pub fn demography_report(
    kingdom: &Kingdom,
    grain_for_peasants: i32,
    grain_for_soldiers: i32,
) -> YearDemography {
    // Original: efficiency = (grain_for_army / army_food_required) * 10, clamped 5-15.
    // No soldiers but grain given → max efficiency (preparing a future army);
    // no soldiers and no grain → default 10 (100%).
    let soldiers_efficiency = if kingdom.soldiers == 0 {
        if grain_for_soldiers > 0 {
            15
        } else {
            10
        }
    } else {
        let army_food_required = kingdom.soldiers_grain_needs().max(1);
        (grain_for_soldiers * 10 / army_food_required).clamp(5, 15)
    };

    YearDemography {
        births: births(kingdom),
        disease_victims: disease_victims(kingdom),
        immigrants: immigrants(kingdom, grain_for_peasants),
        malnutrition_victims: malnutrition_victims(kingdom, grain_for_peasants),
        starvation_victims: starvation_victims(kingdom, grain_for_peasants),
        soldiers_efficiency,
        soldiers_starvation_victims: soldiers_starvation_victims(kingdom, grain_for_soldiers),
        soldiers_desertion_victims: soldiers_desertion_victims(kingdom, grain_for_soldiers),
    }
}

/// Original: IF HS>PD*1.5 THEN D=SQR(HS-PD)-INT(RND*A(K,8)*1.5+1)
///           IF D<0 THEN 81 ELSE DE=INT(RND*(2*D+1))
fn immigrants(kingdom: &Kingdom, grain_for_peasants: i32) -> i32 {
    let food_required = kingdom.peasants_grain_needs();
    if grain_for_peasants > food_required * 3 / 2 {
        let d = ((grain_for_peasants - food_required) as f32).sqrt()
            - random(1, 1 + kingdom.immigration_taxes * 3 / 2) as f32;
        if d <= 0.0 {
            return 0;
        }
        return random(0, (d * 2.0) as i32 + 1);
    }
    0
}

/// Original: DD=INT(RND*PO/22+1) where PO = serfs + merchants + nobles
fn disease_victims(kingdom: &Kingdom) -> i32 {
    random(1, kingdom.population() / 22 + 1)
}

/// Malnutrition only happens when grain given is less than required.
fn malnutrition_victims(kingdom: &Kingdom, grain_for_peasants: i32) -> i32 {
    let food_required = kingdom.peasants_grain_needs();
    if grain_for_peasants < food_required {
        let pop = kingdom.population();
        let shortage_ratio = 1.0 - (grain_for_peasants as f32 / food_required.max(1) as f32);
        let max_victims = (pop as f32 * shortage_ratio / 12.0) as i32 + 1;
        random(1, max_victims.max(2))
    } else {
        0
    }
}

/// Original: DB=INT(RND*PO/9.5+1)
fn births(kingdom: &Kingdom) -> i32 {
    random(1, 1 + kingdom.population() * 10 / 95)
}

/// Starvation happens when grain given is less than 50% of required.
fn starvation_victims(kingdom: &Kingdom, grain_for_peasants: i32) -> i32 {
    let half_required = kingdom.peasants_grain_needs() / 2;
    if grain_for_peasants < half_required {
        let pop = kingdom.population();
        let severity = 1.0 - (grain_for_peasants as f32 / half_required.max(1) as f32);
        let max_victims = (pop as f32 * severity / 16.0) as i32 + 2;
        random(2, max_victims.max(3))
    } else {
        0
    }
}

/// Soldiers lost to starvation when severely underfed (< 50% rations).
fn soldiers_starvation_victims(kingdom: &Kingdom, grain_for_soldiers: i32) -> i32 {
    let half_required = kingdom.soldiers_grain_needs() / 2;
    if grain_for_soldiers < half_required {
        let severity = 1.0 - (grain_for_soldiers as f32 / half_required.max(1) as f32);
        let max_victims = (kingdom.soldiers as f32 * severity / 2.0) as i32 + 1;
        random(1, max_victims.max(2))
    } else {
        0
    }
}

/// Soldiers desert when underfed (>= 50% but < 100% rations).
fn soldiers_desertion_victims(kingdom: &Kingdom, grain_for_soldiers: i32) -> i32 {
    let army_food_required = kingdom.soldiers_grain_needs();
    let half_required = army_food_required / 2;
    if grain_for_soldiers >= half_required && grain_for_soldiers < army_food_required {
        let shortage_ratio = 1.0 - (grain_for_soldiers as f32 / army_food_required.max(1) as f32);
        let max_deserters = (kingdom.soldiers as f32 * shortage_ratio / 5.0) as i32 + 1;
        random(1, max_deserters.max(2))
    } else {
        0
    }
}

/// Feed the population and the army for the year: consumes both grain
/// allocations and applies births, deaths, immigration and army morale.
pub fn apply_feed(
    kingdom: &mut Kingdom,
    grain_for_peasants: i32,
    grain_for_soldiers: i32,
) -> YearDemography {
    let report = demography_report(kingdom, grain_for_peasants, grain_for_soldiers);
    let n = report.immigrants / 25;
    let nobles_immigrants = if n > 0 { random(0, 1 + n) } else { 0 };
    let mut merchants_immigrants = if n > 0 { random(0, 1 + n / 5) } else { 0 };
    merchants_immigrants += random(1, 7);

    kingdom.grain_stocks -= grain_for_peasants + grain_for_soldiers;

    // Original lines 285/288: A(K,3)=A(K,3)+PT-I1 then A(K,3)=A(K,3)-I2
    let peasant_change = report.births + report.immigrants
        - merchants_immigrants
        - nobles_immigrants
        - report.starvation_victims
        - report.malnutrition_victims
        - report.disease_victims;
    kingdom.peasants = max(0, kingdom.peasants + peasant_change);

    let soldier_losses = report.soldiers_starvation_victims + report.soldiers_desertion_victims;
    kingdom.soldiers = max(0, kingdom.soldiers - soldier_losses);
    kingdom.soldiers_efficiency = report.soldiers_efficiency;

    kingdom.nobles += nobles_immigrants;
    kingdom.merchants += merchants_immigrants;

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
        assert_eq!(r.soldiers_efficiency, 10);
    }

    #[test]
    fn starving_population_dies_and_army_deserts() {
        let k = Kingdom::new(Kingdoms::France);
        let r = demography_report(&k, 0, k.soldiers_grain_needs() * 3 / 4);
        assert!(r.starvation_victims > 0);
        assert!(r.malnutrition_victims > 0);
        assert!(r.soldiers_desertion_victims > 0);
        assert_eq!(r.soldiers_starvation_victims, 0);
        assert_eq!(r.soldiers_efficiency, 7);
    }

    #[test]
    fn efficiency_is_clamped_and_handles_empty_army() {
        let mut k = Kingdom::new(Kingdoms::France);
        assert_eq!(demography_report(&k, 0, 1_000_000).soldiers_efficiency, 15);
        assert_eq!(demography_report(&k, 0, 0).soldiers_efficiency, 5);
        k.soldiers = 0;
        assert_eq!(demography_report(&k, 0, 10).soldiers_efficiency, 15);
        assert_eq!(demography_report(&k, 0, 0).soldiers_efficiency, 10);
    }

    #[test]
    fn apply_feed_consumes_grain_once() {
        let mut k = Kingdom::new(Kingdoms::France);
        let before = k.grain_stocks;
        apply_feed(&mut k, 1000, 200);
        assert_eq!(k.grain_stocks, before - 1200);
        assert!(k.peasants >= 0 && k.soldiers >= 0);
    }
}
