use crate::game::EmpireGame;
use crate::kingdom::Kingdoms;

/// Maximum grain price per bushel (original: 15).
pub const MAX_GRAIN_PRICE: i32 = 15;

/// Land sell price per arpent (original: 2 currency).
pub const LAND_SELL_PRICE: i32 = 2;

pub enum Trade {
    /// Buy `amount` bushels from `seller` at the seller's listed price.
    Buy {
        amount: i32,
        seller: Kingdoms,
    },
    /// Put `amount` bushels up for sale at `price` per bushel.
    Sell {
        amount: i32,
        price: i32,
    },
    /// Sell `arpents` of land to the barbarians.
    SellLand {
        arpents: i32,
    },
    None,
}

/// Total cost for buying grain including the 10% broker fee.
/// Original formula: cost = amount * seller_price / 0.9
pub fn calculate_buy_cost(amount: i32, price_per_unit: i32) -> i32 {
    amount * price_per_unit * 10 / 9
}

pub fn apply_trade(game: &mut EmpireGame, buyer: Kingdoms, trade: Trade) {
    match trade {
        Trade::Buy { amount, seller } => {
            let price = game.kingdom(seller).grain_price.min(MAX_GRAIN_PRICE);
            let cost = calculate_buy_cost(amount, price);
            let k = game.kingdom_mut(buyer);
            k.grain_stocks += amount;
            k.treasury -= cost;
        }
        Trade::Sell { amount, price } => {
            let price = price.min(MAX_GRAIN_PRICE);
            let k = game.kingdom_mut(buyer);
            // Original: A(K,6)=(A(K,6)*A(K,5)+H1*H2)/(A(K,5)+H1) — weighted average price
            let new_total = k.grain_to_sell + amount;
            k.grain_price = if new_total > 0 {
                (k.grain_price * k.grain_to_sell + price * amount) / new_total
            } else {
                price
            };
            k.grain_to_sell = new_total;
            k.grain_stocks -= amount;
        }
        Trade::SellLand { arpents } => {
            let k = game.kingdom_mut(buyer);
            k.surface -= arpents;
            k.treasury += arpents * LAND_SELL_PRICE;
        }
        Trade::None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_cost_includes_broker_fee() {
        assert_eq!(calculate_buy_cost(90, 10), 1000);
    }

    #[test]
    fn buying_uses_sellers_capped_price() {
        let mut game = EmpireGame::default();
        game.kingdom_mut(Kingdoms::Spain).grain_price = 40; // capped to 15
        let before = game.kingdom(Kingdoms::France).clone();
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::Buy {
                amount: 9,
                seller: Kingdoms::Spain,
            },
        );
        let after = game.kingdom(Kingdoms::France);
        assert_eq!(after.grain_stocks, before.grain_stocks + 9);
        assert_eq!(after.treasury, before.treasury - 150);
    }

    #[test]
    fn selling_averages_the_price() {
        let mut game = EmpireGame::default();
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::Sell {
                amount: 100,
                price: 10,
            },
        );
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::Sell {
                amount: 100,
                price: 20,
            },
        );
        let k = game.kingdom(Kingdoms::France);
        assert_eq!(k.grain_to_sell, 200);
        assert_eq!(k.grain_price, 12); // second sale capped to 15 → (1000+1500)/200
    }

    #[test]
    fn selling_land_pays_two_per_arpent() {
        let mut game = EmpireGame::default();
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::SellLand { arpents: 100 },
        );
        let k = game.kingdom(Kingdoms::France);
        assert_eq!(k.surface, 9900);
        assert_eq!(k.treasury, 1200);
    }
}
