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
}

impl InvestmentType {
    /// Cost per unit.
    pub fn cost(self) -> i32 {
        match self {
            InvestmentType::Marketplaces => 1000,
            InvestmentType::GrainMills => 2000,
            InvestmentType::Foundries => 7000,
            InvestmentType::Shipyards => 8000,
            InvestmentType::Soldiers => 8,
            InvestmentType::Palaces => 5000,
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
            _ => None,
        }
    }

    /// Maximum amount the kingdom can afford (and, for soldiers, command).
    pub fn max_investment(self, kingdom: &Kingdom) -> i32 {
        let max_by_treasury = kingdom.treasury / self.cost();
        match self {
            InvestmentType::Soldiers => {
                let max_by_nobles = kingdom.nobles * 20 - kingdom.soldiers;
                min(max_by_treasury, max_by_nobles).max(0)
            }
            _ => max_by_treasury,
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

    if total_cost > kingdom.treasury {
        return InvestmentResult {
            amount,
            total_cost,
            error: Some(format!(
                "Insufficient funds: need {} but only have {}",
                total_cost, kingdom.treasury
            )),
            ..Default::default()
        };
    }

    if investment_type == InvestmentType::Soldiers {
        let max_soldiers = kingdom.nobles * 20;
        if kingdom.soldiers + amount > max_soldiers {
            return InvestmentResult {
                amount,
                total_cost,
                error: Some(format!(
                    "Your {} nobles can only command up to {} soldiers",
                    kingdom.nobles, max_soldiers
                )),
                ..Default::default()
            };
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

/// Recruit soldiers from the peasantry, paying the per-soldier cost.
pub fn recruit_soldiers(kingdom: &mut Kingdom, count: i32) -> Result<(), String> {
    if count <= 0 {
        return Ok(());
    }

    let cost = InvestmentType::Soldiers.cost() * count;
    if kingdom.treasury < cost {
        return Err(format!(
            "Insufficient treasury: need {} but only have {}",
            cost, kingdom.treasury
        ));
    }

    let max_soldiers = kingdom.nobles * 20;
    if kingdom.soldiers + count > max_soldiers {
        return Err(format!(
            "Your {} nobles can only command up to {} soldiers (currently have {})",
            kingdom.nobles, max_soldiers, kingdom.soldiers
        ));
    }

    if kingdom.peasants < count {
        return Err(format!(
            "Not enough peasants to recruit: need {} but only have {}",
            count, kingdom.peasants
        ));
    }

    kingdom.treasury -= cost;
    kingdom.peasants -= count;
    kingdom.soldiers += count;
    Ok(())
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
    fn recruit_soldiers_validates_and_applies() {
        let mut k = Kingdom::new(Kingdoms::France);
        k.nobles = 5;
        assert!(recruit_soldiers(&mut k, 0).is_ok());
        assert!(recruit_soldiers(&mut k, 1000).is_err()); // cost 8000 > 1000
        assert!(recruit_soldiers(&mut k, 100).is_err()); // 120 > 5*20
        recruit_soldiers(&mut k, 50).unwrap();
        assert_eq!(k.soldiers, 70);
        assert_eq!(k.peasants, 1950);
        assert_eq!(k.treasury, 600);
    }
}
