use crate::kingdom::Kingdom;
use crate::random::random;
use crate::weather::Weather;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxType {
    Immigration,
    Commercial,
    Income,
}

impl TaxType {
    /// Maximum tax rate allowed per original Empire.bas specs.
    pub fn max_rate(self) -> i32 {
        match self {
            TaxType::Immigration => 50,
            TaxType::Commercial => 20,
            TaxType::Income => 35,
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

/// Original: F1 = (marketplaces * ((merchants + 2d35) / (sales_tax + 1) * 12 + 5)) ^ 0.9
fn marketplaces_profit(kingdom: &Kingdom) -> i32 {
    let marketplaces = kingdom.marketplaces as f32;
    let merchants = kingdom.merchants as f32;
    let sales_tax = (kingdom.commercial_taxes + 1) as f32;
    let random_factor = (random(1, 35) + random(1, 35)) as f32;
    let base = marketplaces * ((merchants + random_factor) / sales_tax * 12.0 + 5.0);
    base.powf(0.9) as i32
}

/// Original: F2 = (mills * (5.8 * (harvest + random(250)) / (income_tax * 20 + sales_tax * 40 + 10) + 150)) ^ 0.9
fn grain_mills_profit(kingdom: &Kingdom) -> i32 {
    let mills = kingdom.grain_mills as f32;
    let harvest = kingdom.grain_harvest as f32;
    let random_factor = random(1, 250) as f32;
    let divisor =
        kingdom.income_taxes as f32 * 20.0 + kingdom.commercial_taxes as f32 * 40.0 + 10.0;
    let base = mills * (5.8 * (harvest + random_factor) / divisor + 150.0);
    base.powf(0.9) as i32
}

/// Original: F3 = (foundries + (soldiers + random(150) + 400)) ^ 0.9
fn foundries_profit(kingdom: &Kingdom) -> i32 {
    let base = (kingdom.foundries + kingdom.soldiers + random(1, 150) + 400) as f32;
    base.powf(0.9) as i32
}

/// Original: F4 = (shipyards * (merchants * 4 + marketplaces * 9 + foundries * 15) * weather) ^ 0.9
fn shipyards_profit(kingdom: &Kingdom, weather: Weather) -> i32 {
    let base = kingdom.shipyards as f32
        * (kingdom.merchants as f32 * 4.0
            + kingdom.marketplaces as f32 * 9.0
            + kingdom.foundries as f32 * 15.0)
        * weather.value() as f32;
    base.powf(0.9) as i32
}

fn soldiers_maintenance(kingdom: &Kingdom) -> i32 {
    kingdom.soldiers * 8
}

/// Original: FC=DE*(2d40)/100*A(K,8)
fn immigration_taxes_profits(kingdom: &Kingdom, immigration: i32) -> i32 {
    let random_factor = random(1, 40) + random(1, 40);
    immigration * random_factor / 100 * kingdom.immigration_taxes
}

fn commercial_taxes_profits(
    kingdom: &Kingdom,
    marketplaces_profit: i32,
    grain_mills_profit: i32,
    foundries_profit: i32,
    shipyards_profit: i32,
) -> i32 {
    let tax = kingdom.commercial_taxes as f32 / 100.0;
    let trade = (kingdom.merchants * 18 / 10
        + marketplaces_profit * 33
        + grain_mills_profit * 17
        + foundries_profit * 50
        + shipyards_profit * 70) as f32;
    let taxable = trade.powf(0.85) as i32 + kingdom.nobles * 5 + kingdom.peasants;
    (tax * taxable as f32) as i32
}

/// Original: FI=(A(K,10)/100*(A(K,3)*1.3+A(K,18)*145+A(K,7)*39+A(K,11)*99+A(K,12)*99+A(K,13)*425+A(K,14)*965))^.97
fn income_taxes_profits(kingdom: &Kingdom) -> i32 {
    let tax_rate = kingdom.income_taxes as f32 / 100.0;
    let base = kingdom.peasants as f32 * 1.3
        + kingdom.nobles as f32 * 145.0
        + kingdom.merchants as f32 * 39.0
        + kingdom.marketplaces as f32 * 99.0
        + kingdom.grain_mills as f32 * 99.0
        + kingdom.foundries as f32 * 425.0
        + kingdom.shipyards as f32 * 965.0;
    (tax_rate * base).powf(0.97) as i32
}

pub fn economy_report(kingdom: &Kingdom, weather: Weather, immigration: i32) -> YearEconomy {
    let marketplaces_profits = marketplaces_profit(kingdom);
    let grain_mills_profits = grain_mills_profit(kingdom);
    let foundries_profits = foundries_profit(kingdom);
    let shipyards_profits = shipyards_profit(kingdom, weather);
    YearEconomy {
        marketplaces_profits,
        grain_mills_profits,
        foundries_profits,
        shipyards_profits,
        soldiers_maintenance: soldiers_maintenance(kingdom),
        immigration_taxes_profits: immigration_taxes_profits(kingdom, immigration),
        commercial_taxes_profits: commercial_taxes_profits(
            kingdom,
            marketplaces_profits,
            grain_mills_profits,
            foundries_profits,
            shipyards_profits,
        ),
        income_taxes_profits: income_taxes_profits(kingdom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Kingdoms;

    #[test]
    fn tax_change_is_clamped() {
        let mut k = Kingdom::new(Kingdoms::France);
        apply_tax_change(&mut k, TaxType::Commercial, 99);
        assert_eq!(k.commercial_taxes, 20);
        apply_tax_change(&mut k, TaxType::Income, -5);
        assert_eq!(k.income_taxes, 0);
        apply_tax_change(&mut k, TaxType::Immigration, 30);
        assert_eq!(k.immigration_taxes, 30);
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
}
