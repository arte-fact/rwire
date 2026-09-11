use std::cmp::min;

use crate::kingdom::Kingdom;
use crate::random::random;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvestmentType {
    Marketplaces,
    GrainMills,
    Foundries,
    Shipyards,
    Soldiers,
    Palaces,
    Fortifications,
    Hospices,
    Rams,
    Scouts,
}

/// A palace, fortifications and a hospice are built by tenths, up to ten.
pub const TENTHS: i32 = 10;

impl InvestmentType {
    /// Cost per unit.
    pub const fn cost(self) -> i32 {
        match self {
            InvestmentType::Marketplaces => 1000,
            InvestmentType::GrainMills => 2000,
            InvestmentType::Foundries => 7000,
            InvestmentType::Shipyards => 8000,
            InvestmentType::Soldiers => 8,
            InvestmentType::Palaces => 5000,
            InvestmentType::Fortifications => 1000,
            InvestmentType::Hospices => 5000,
            InvestmentType::Rams => 1500,
            InvestmentType::Scouts => 16,
        }
    }

    /// Where the kingdom stands on a purchase bought by tenths.
    pub fn tenths(self, kingdom: &Kingdom) -> Option<i32> {
        match self {
            InvestmentType::Palaces => Some(kingdom.palaces),
            InvestmentType::Fortifications => Some(kingdom.fortifications),
            InvestmentType::Hospices => Some(kingdom.hospices),
            _ => None,
        }
    }

    pub fn from_number(n: i32) -> Option<InvestmentType> {
        match n {
            1 => Some(InvestmentType::Marketplaces),
            2 => Some(InvestmentType::GrainMills),
            3 => Some(InvestmentType::Foundries),
            4 => Some(InvestmentType::Shipyards),
            5 => Some(InvestmentType::Soldiers),
            6 => Some(InvestmentType::Palaces),
            7 => Some(InvestmentType::Fortifications),
            8 => Some(InvestmentType::Hospices),
            9 => Some(InvestmentType::Rams),
            10 => Some(InvestmentType::Scouts),
            _ => None,
        }
    }

    /// Maximum amount the kingdom can afford (and, for soldiers, command:
    /// twenty men a noble; for the tenths, build: ten). Recruits eat nothing
    /// when hired; the council feeds them with the rest of the army.
    pub fn max_investment(self, kingdom: &Kingdom) -> i32 {
        let max_by_treasury = kingdom.treasury / self.cost();
        match self {
            InvestmentType::Soldiers => {
                let max_by_nobles = kingdom.nobles * 20 - kingdom.soldiers;
                min(max_by_treasury, max_by_nobles).max(0)
            }
            _ => match self.tenths(kingdom) {
                Some(built) => min(max_by_treasury, TENTHS - built).max(0),
                None => max_by_treasury,
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct InvestmentResult {
    pub success: bool,
    pub amount: i32,
    pub total_cost: i32,
    pub error: Option<String>,
    pub side_effects: InvestmentSideEffects,
}

/// Side effects from investments (per original Empire.bas).
#[derive(Debug, Default)]
pub struct InvestmentSideEffects {
    /// Merchants attracted by building marketplaces: random(1-7) per marketplace.
    pub merchants_attracted: i32,
    /// Nobles attracted by building palace: random(1-4) per 10%.
    pub nobles_attracted: i32,
}

pub fn validate_investment(
    kingdom: &Kingdom,
    investment_type: InvestmentType,
    amount: i32,
) -> InvestmentResult {
    let total_cost = amount * investment_type.cost();
    let failed = |error: String| InvestmentResult {
        amount,
        total_cost,
        error: Some(error),
        ..Default::default()
    };

    if total_cost > kingdom.treasury {
        return failed(format!(
            "Insufficient funds: need {} but only have {}",
            total_cost, kingdom.treasury
        ));
    }

    if investment_type == InvestmentType::Soldiers {
        let max_soldiers = kingdom.nobles * 20;
        if kingdom.soldiers + amount > max_soldiers {
            return failed(format!(
                "Your {} nobles can only command up to {} soldiers",
                kingdom.nobles, max_soldiers
            ));
        }
    }

    if let Some(built) = investment_type.tenths(kingdom) {
        if built + amount > TENTHS {
            return failed(format!(
                "Only {} tenths left to build",
                (TENTHS - built).max(0)
            ));
        }
    }

    InvestmentResult {
        success: true,
        amount,
        total_cost,
        ..Default::default()
    }
}

/// Apply an investment. Per original Empire.bas some investments have side
/// effects: marketplaces attract merchants (drawn from serfs) and palaces
/// attract nobles.
pub fn apply_investment(
    kingdom: &mut Kingdom,
    investment_type: InvestmentType,
    amount: i32,
) -> InvestmentResult {
    let validation = validate_investment(kingdom, investment_type, amount);
    if !validation.success {
        return validation;
    }

    let mut side_effects = InvestmentSideEffects::default();

    match investment_type {
        InvestmentType::Marketplaces => {
            kingdom.marketplaces += amount;
            let merchants_gained = (0..amount).map(|_| random(1, 7)).sum::<i32>();
            kingdom.merchants += merchants_gained;
            kingdom.peasants -= merchants_gained;
            side_effects.merchants_attracted = merchants_gained;
        }
        InvestmentType::GrainMills => kingdom.grain_mills += amount,
        InvestmentType::Foundries => kingdom.foundries += amount,
        InvestmentType::Shipyards => kingdom.shipyards += amount,
        InvestmentType::Soldiers => kingdom.soldiers += amount,
        InvestmentType::Palaces => {
            kingdom.palaces += amount;
            let nobles_gained = (0..amount).map(|_| random(1, 4)).sum::<i32>();
            kingdom.nobles += nobles_gained;
            side_effects.nobles_attracted = nobles_gained;
        }
        InvestmentType::Fortifications => kingdom.fortifications += amount,
        InvestmentType::Hospices => kingdom.hospices += amount,
        InvestmentType::Rams => kingdom.rams += amount,
        InvestmentType::Scouts => kingdom.scouts += amount,
    }

    kingdom.treasury -= validation.total_cost;

    InvestmentResult {
        success: true,
        amount,
        total_cost: validation.total_cost,
        error: None,
        side_effects,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kingdom::Kingdoms;

    #[test]
    fn insufficient_funds_is_rejected_without_side_effects() {
        let mut k = Kingdom::new(Kingdoms::France);
        let r = apply_investment(&mut k, InvestmentType::Foundries, 1);
        assert!(!r.success);
        assert!(r.error.unwrap().starts_with("Insufficient funds"));
        assert_eq!(k.foundries, 0);
        assert_eq!(k.treasury, 1000);
    }

    #[test]
    fn marketplace_converts_serfs_to_merchants() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.treasury = 5000;
        let r = apply_investment(&mut k, InvestmentType::Marketplaces, 2);
        assert!(r.success);
        assert_eq!(k.marketplaces, 2);
        assert_eq!(k.treasury, 3000);
        let gained = r.side_effects.merchants_attracted;
        assert!((2..=12).contains(&gained));
        assert_eq!(k.merchants, 25 + gained);
        assert_eq!(k.peasants, 2000 - gained);
    }

    #[test]
    fn soldiers_are_capped_by_nobles() {
        let k = Kingdom::new(Kingdoms::France);
        // 1 noble → 20 soldiers max, already have 20
        assert_eq!(InvestmentType::Soldiers.max_investment(&k), 0);
        let r = validate_investment(&k, InvestmentType::Soldiers, 1);
        assert!(!r.success);
    }

    #[test]
    fn recruits_cost_money_only_and_are_led_by_the_nobles() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 5;
        k.soldiers_ration = 120;
        k.grain_stocks = 500;
        // 1000 francs buy 125 men; the nobles command 80 more; grain is no bar.
        assert_eq!(InvestmentType::Soldiers.max_investment(&k), 80);
        let r = apply_investment(&mut k, InvestmentType::Soldiers, 40);
        assert!(r.success);
        assert_eq!(k.soldiers, 60);
        assert_eq!(k.treasury, 680);
        assert_eq!(k.grain_stocks, 500);
    }

    #[test]
    fn tenths_stop_at_ten() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.treasury = 100_000;
        k.fortifications = 8;
        assert_eq!(InvestmentType::Fortifications.max_investment(&k), 2);
        assert!(!apply_investment(&mut k, InvestmentType::Fortifications, 3).success);
        assert!(apply_investment(&mut k, InvestmentType::Fortifications, 2).success);
        assert_eq!(k.fortifications, 10);
        assert_eq!(k.treasury, 98_000);
        assert_eq!(InvestmentType::Hospices.max_investment(&k), 10);
        assert_eq!(InvestmentType::Palaces.max_investment(&k), 10);
    }

    #[test]
    fn rams_and_scouts_go_to_stock() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.treasury = 3016;
        assert!(apply_investment(&mut k, InvestmentType::Rams, 2).success);
        assert!(apply_investment(&mut k, InvestmentType::Scouts, 1).success);
        assert_eq!((k.rams, k.scouts, k.treasury), (2, 1, 0));
    }
}
