use crate::game::EmpireGame;
#[cfg(doc)]
use crate::kingdom::Kingdom;
use crate::kingdom::Kingdoms;

/// Grain is priced by the hundred bushels ("le cent"), so that a kingdom's
/// yearly surplus of some eight thousand bushels is worth a couple of years of
/// taxes at the usual 10–40, not a hundred times the treasury as it was at the
/// original's 1–15 a bushel.
pub const GRAIN_LOT: i32 = 100;

/// Maximum grain price per hundred bushels: one franc a bushel.
pub const MAX_GRAIN_PRICE: i32 = 100;

/// Land sell price per arpent: selling the yearly tenth of a starting kingdom
/// (a thousand arpents) pays a few years of taxes, or one drought's grain at a
/// famine price. The original paid 2.
pub const LAND_SELL_PRICE: i32 = 4;

/// The most land a kingdom may sell in a year: a tenth of its surface.
pub fn max_land_sale(surface: i32) -> i32 {
    surface / 10
}

pub enum Trade {
    /// Buy `amount` bushels from `seller` at the seller's listed price.
    Buy {
        amount: i32,
        seller: Kingdoms,
    },
    /// Offer `amount` bushels at `price` per hundred bushels; they stay in the
    /// stocks and reach the stall next year (see [`Kingdom::open_market`]).
    Sell {
        amount: i32,
        price: i32,
    },
    /// Sell `arpents` of land to the barbarians (capped by [`max_land_sale`]).
    SellLand {
        arpents: i32,
    },
    None,
}

/// What the seller gets for `amount` bushels listed at `price` the hundred.
pub fn grain_value(amount: i32, price: i32) -> i32 {
    amount * price / GRAIN_LOT
}

/// Total cost for buying grain including the 10 % broker fee, rounded up.
/// Original formula: cost = amount * seller_price / 0.9
pub fn calculate_buy_cost(amount: i32, price: i32) -> i32 {
    (amount * price * 10 + 9 * GRAIN_LOT - 1) / (9 * GRAIN_LOT)
}

pub fn apply_trade(game: &mut EmpireGame, buyer: Kingdoms, trade: Trade) {
    match trade {
        Trade::Buy { amount, seller } => {
            let s = game.kingdom_mut(seller);
            let price = s.grain_price.min(MAX_GRAIN_PRICE);
            let amount = amount.clamp(0, s.for_sale());
            // The seller receives the base price; the 10% broker fee is lost.
            s.grain_to_sell -= amount;
            s.grain_stocks -= amount;
            s.treasury += grain_value(amount, price);
            let cost = calculate_buy_cost(amount, price);
            let k = game.kingdom_mut(buyer);
            k.grain_stocks += amount;
            k.treasury -= cost;
        }
        Trade::Sell { amount, price } => {
            game.kingdom_mut(buyer)
                .list_grain(amount, price.min(MAX_GRAIN_PRICE));
        }
        Trade::SellLand { arpents } => {
            let k = game.kingdom_mut(buyer);
            let arpents = arpents.clamp(0, max_land_sale(k.surface));
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
        assert_eq!(calculate_buy_cost(9000, 10), 1000);
        // Rounded up: nine bushels at a franc each cost ten.
        assert_eq!(calculate_buy_cost(9, 100), 10);
        assert_eq!(grain_value(9, 100), 9);
    }

    #[test]
    fn buying_uses_sellers_capped_price_and_pays_the_seller() {
        let mut game = EmpireGame::default();
        let spain = game.kingdom_mut(Kingdoms::Spain);
        spain.grain_price = 250; // capped to 100
        spain.grain_to_sell = 1000;
        let spain_stocks = spain.grain_stocks;
        let before = game.kingdom(Kingdoms::France).clone();
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::Buy {
                amount: 900,
                seller: Kingdoms::Spain,
            },
        );
        let after = game.kingdom(Kingdoms::France);
        assert_eq!(after.grain_stocks, before.grain_stocks + 900);
        assert_eq!(after.treasury, before.treasury - 1000);
        let spain = game.kingdom(Kingdoms::Spain);
        assert_eq!(spain.grain_to_sell, 100);
        assert_eq!(spain.grain_stocks, spain_stocks - 900);
        assert_eq!(spain.treasury, 1000 + 900);
    }

    #[test]
    fn buying_is_capped_to_the_sellers_stocks() {
        let mut game = EmpireGame::default();
        let spain = game.kingdom_mut(Kingdoms::Spain);
        spain.grain_price = 10;
        spain.grain_to_sell = 1000;
        spain.grain_stocks = 300; // the rats got the rest
        assert_eq!(spain.for_sale(), 300);
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::Buy {
                amount: 900,
                seller: Kingdoms::Spain,
            },
        );
        let spain = game.kingdom(Kingdoms::Spain);
        assert_eq!((spain.grain_stocks, spain.grain_to_sell), (0, 700));
        assert_eq!(spain.treasury, 1000 + 30);
        // Opening the market trims the stall to the stocks.
        game.kingdom_mut(Kingdoms::Spain).open_market();
        assert_eq!(game.kingdom(Kingdoms::Spain).grain_to_sell, 0);
    }

    #[test]
    fn buying_is_capped_to_what_is_on_the_market() {
        let mut game = EmpireGame::default();
        let before = game.kingdom(Kingdoms::France).clone();
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::Buy {
                amount: 500,
                seller: Kingdoms::Spain, // nothing for sale
            },
        );
        assert_eq!(game.kingdom(Kingdoms::France), &before);
    }

    #[test]
    fn selling_averages_the_price() {
        let mut game = EmpireGame::default();
        let stocks = game.kingdom(Kingdoms::France).grain_stocks;
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
                price: 140,
            },
        );
        let k = game.kingdom(Kingdoms::France);
        // Listed grain stays in the stocks and is not on the stall yet.
        assert_eq!(k.grain_stocks, stocks);
        assert_eq!(k.grain_to_sell, 0);
        assert_eq!(k.listing, Some((200, 55))); // second sale capped to 100 → (1000+10000)/200
        game.kingdom_mut(Kingdoms::France).open_market();
        let k = game.kingdom(Kingdoms::France);
        assert_eq!(k.listing, None);
        assert_eq!(k.grain_to_sell, 200);
        assert_eq!(k.grain_price, 55);
    }

    #[test]
    fn opening_the_market_averages_with_unsold_grain() {
        let mut game = EmpireGame::default();
        let k = game.kingdom_mut(Kingdoms::France);
        k.grain_to_sell = 300;
        k.grain_price = 20;
        let stocks = k.grain_stocks;
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::Sell {
                amount: 100,
                price: 60,
            },
        );
        let k = game.kingdom_mut(Kingdoms::France);
        assert_eq!(k.grain_stocks, stocks);
        assert_eq!((k.grain_to_sell, k.grain_price), (300, 20));
        k.open_market();
        assert_eq!((k.grain_to_sell, k.grain_price), (400, 30));
        // Nothing listed: the stall is left alone.
        k.open_market();
        assert_eq!((k.grain_to_sell, k.grain_price), (400, 30));
    }

    #[test]
    fn selling_land_pays() {
        let mut game = EmpireGame::default();
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::SellLand { arpents: 100 },
        );
        let k = game.kingdom(Kingdoms::France);
        assert_eq!(k.surface, 9900);
        assert_eq!(k.treasury, 1000 + 100 * LAND_SELL_PRICE);
    }

    #[test]
    fn land_sales_are_capped_to_a_tenth_of_the_surface() {
        let mut game = EmpireGame::default();
        apply_trade(
            &mut game,
            Kingdoms::France,
            Trade::SellLand { arpents: 5000 },
        );
        let k = game.kingdom(Kingdoms::France);
        assert_eq!(k.surface, 9000);
        assert_eq!(k.treasury, 1000 + 1000 * LAND_SELL_PRICE);
        assert_eq!(max_land_sale(9000), 900);
    }

    #[test]
    fn offers_are_capped_to_the_stocks() {
        let mut game = EmpireGame::default();
        let k = game.kingdom_mut(Kingdoms::France);
        k.grain_stocks = 1000;
        k.grain_to_sell = 400;
        k.list_grain(900, 10);
        assert_eq!(k.listing, Some((600, 10)));
        k.list_grain(100, 10);
        assert_eq!(k.listing, Some((600, 10)));
        assert_eq!(k.offered(), 1000);
    }
}
