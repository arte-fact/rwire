use std::cmp::max;

use crate::kingdom::Kingdom;
use crate::random::random;
use crate::weather::Weather;

pub fn grain_harvest(kingdom: &Kingdom, weather: Weather) -> i32 {
    kingdom.cultivated_surface() * weather.value() * 72 / 100 + random(1, 500)
        - kingdom.foundries * 500
}

pub fn apply_grain_harvest(kingdom: &mut Kingdom, weather: Weather) {
    kingdom.grain_harvest = grain_harvest(kingdom, weather);
    kingdom.grain_stocks += kingdom.grain_harvest;
}

/// Seed the fields: consumes one bushel per three arpents actually sown.
pub fn apply_seed_grain(kingdom: &mut Kingdom) {
    let mut surface = kingdom.cultivated_surface();

    // Original: IF A(K,2)*3<LA THEN LA=A(K,2)*3
    if kingdom.grain_stocks * 3 < surface {
        surface = kingdom.grain_stocks * 3;
    }

    // Original: IF A(K,3)*5<LA THEN LA=A(K,3)*5
    if kingdom.peasants * 5 < surface {
        surface = kingdom.peasants * 5;
    }

    // Original: A(K,2)=A(K,2)-LA/3
    kingdom.grain_stocks -= surface / 3;
}

/// The rats take their share of the granaries and of the stall alike.
pub fn apply_rat_loss_rate(kingdom: &mut Kingdom) {
    let loss_rate = random(1, 30);
    let gnawed = |grain: i32| max(0, grain - grain * loss_rate / 100);
    kingdom.grain_stocks = gnawed(kingdom.grain_stocks);
    kingdom.grain_to_sell = gnawed(kingdom.grain_to_sell);
    kingdom.rats_loss_rate = loss_rate;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Kingdoms;

    #[test]
    fn seed_grain_is_limited_by_stocks_and_peasants() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.grain_stocks = 100;
        apply_seed_grain(&mut k);
        // surface capped to 300 → consumes 100
        assert_eq!(k.grain_stocks, 0);

        let mut k = Kingdom::new(Kingdoms::France);
        k.grain_stocks = 100_000;
        k.peasants = 10;
        let before = k.grain_stocks;
        apply_seed_grain(&mut k);
        // surface capped to peasants*5 = 50 → consumes 16
        assert_eq!(before - k.grain_stocks, 50 / 3);
    }

    #[test]
    fn rats_gnaw_the_stall_too() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.grain_stocks = 10_000;
        k.grain_to_sell = 10_000;
        apply_rat_loss_rate(&mut k);
        assert_eq!(k.grain_to_sell, k.grain_stocks);
        assert_eq!(k.grain_to_sell, 10_000 - 100 * k.rats_loss_rate);
    }

    #[test]
    fn rats_never_push_stocks_negative() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.grain_stocks = 0;
        apply_rat_loss_rate(&mut k);
        assert_eq!(k.grain_stocks, 0);
        assert!((1..30).contains(&k.rats_loss_rate));
    }

    #[test]
    fn harvest_adds_to_stocks() {
        let mut k = Kingdom::new(Kingdoms::France);
        let before = k.grain_stocks;
        apply_grain_harvest(&mut k, Weather::Good);
        assert_eq!(k.grain_stocks, before + k.grain_harvest);
        assert!(k.grain_harvest > 0);
    }
}
