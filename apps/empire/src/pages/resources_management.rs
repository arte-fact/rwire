use empire_lib::trade::{max_land_sale, Trade, LAND_SELL_PRICE, MAX_GRAIN_PRICE};
use empire_lib::{EmpireGame, Kingdom, Kingdoms, KINGDOMS};

use crate::ui::{
    black_on_gray, bottom_input_number_with_range, bottom_input_number_with_range_or_enter,
    clear_screen, large_number, print_at,
};

fn draw_resource_management_page(game: &EmpireGame, id: Kingdoms) {
    let kingdom = game.kingdom(id);
    clear_screen();

    print_at(1, 30, &kingdom.full_title());
    print_at(2, 37, &format!("An {}", game.year));
    print_at(
        3,
        16,
        &format!(
            "Les rats ont mangé {} % de vos réserves de grain",
            kingdom.rats_loss_rate
        ),
    );
    print_at(
        4,
        0,
        "╔═══════════════╤═══════════════╤═══════════════╤═══════════════╤═════════════╗",
    );
    print_at(
        5,
        0,
        "║  Récoltes     │   Réserves    │    Besoins    │    Besoins    │    Trésor   ║",
    );
    print_at(
        6,
        0,
        "║  de grain     │   de grain    │   du peuple   │   de l'ost    │    royal    ║",
    );
    print_at(
        7,
        0,
        &format!(
            "║  {:>8}     │   {:>8}    │    {:>8}   │   {:>8}    │  {:>8}   ║",
            large_number(kingdom.grain_harvest),
            large_number(kingdom.grain_stocks),
            large_number(kingdom.peasants_grain_needs()),
            large_number(kingdom.soldiers_grain_needs()),
            large_number(kingdom.treasury)
        ),
    );
    print_at(
        8,
        0,
        &format!(
            "║  boisseaux    │  boisseaux    │   boisseaux   │  boisseaux    │  {:>8}   ║",
            kingdom.currency()
        ),
    );
    print_at(
        9,
        0,
        "╚═══════════════╧═══════════════╧═══════════════╧═══════════════╧═════════════╝",
    );
    print_at(11, 15, "* * * Grain à vendre :");
    print_at(12, 29, &black_on_gray("Pays            Boisseaux    Prix"));
    for (i, seller) in KINGDOMS.iter().enumerate() {
        let s = game.kingdom(*seller);
        print_at(
            13 + i,
            17,
            &format!(
                "{:<2}          {:<16} {:>8}{:>8}",
                i + 1,
                s.name(),
                large_number(s.grain_to_sell),
                s.grain_price.min(MAX_GRAIN_PRICE)
            ),
        );
    }
}

fn peasants_grain_range(kingdom: &Kingdom) -> (i32, i32) {
    (kingdom.grain_stocks / 10, kingdom.grain_stocks)
}

pub fn prompt_grain_for_peasants(game: &EmpireGame, id: Kingdoms) -> i32 {
    draw_resource_management_page(game, id);
    let kingdom = game.kingdom(id);
    print_at(
        20,
        0,
        &format!(
            "Combien de grain donnerez-vous aux {} habitants ?",
            kingdom.population()
        ),
    );
    let (min, max) = peasants_grain_range(kingdom);
    bottom_input_number_with_range(None, min.min(max), max.max(0))
}

pub fn prompt_grain_for_soldiers(game: &EmpireGame, id: Kingdoms) -> i32 {
    draw_resource_management_page(game, id);
    let kingdom = game.kingdom(id);
    print_at(
        20,
        0,
        &format!(
            "Combien de grain souhaitez-vous donner à votre ost de {} hommes ?",
            kingdom.soldiers
        ),
    );
    bottom_input_number_with_range(None, 0, kingdom.grain_stocks.max(0))
}

pub fn prompt_trade_option(game: &EmpireGame, id: Kingdoms) -> Trade {
    loop {
        draw_resource_management_page(game, id);
        print_at(
            20,
            0,
            " ↵ ou 1=Achat de grain   2=Vente de grain   3=Vente de terres :",
        );

        match bottom_input_number_with_range_or_enter(None, 1, 3) {
            Some(1) => {
                if let Some(trade) = prompt_grain_to_buy(game, id) {
                    return trade;
                }
            }
            Some(2) => return prompt_grain_to_sell(game, id),
            Some(3) => return prompt_land_to_sell(game, id),
            _ => return Trade::None,
        }
    }
}

fn prompt_land_to_sell(game: &EmpireGame, id: Kingdoms) -> Trade {
    draw_resource_management_page(game, id);
    Trade::SellLand {
        arpents: bottom_input_number_with_range(
            Some(&format!(
                "A raison de {LAND_SELL_PRICE} francs l'arpent, combien en vendez-vous aux Barbares (un dixième au plus) ?"
            )),
            1,
            max_land_sale(game.kingdom(id).surface).max(1),
        ),
    }
}

fn prompt_grain_to_sell(game: &EmpireGame, id: Kingdoms) -> Trade {
    draw_resource_management_page(game, id);
    Trade::Sell {
        amount: bottom_input_number_with_range(
            Some("Combien de boisseaux vendez-vous ?"),
            1,
            game.kingdom(id).grain_stocks.max(1),
        ),
        price: bottom_input_number_with_range(
            Some("A quel prix le boisseau ?"),
            1,
            MAX_GRAIN_PRICE,
        ),
    }
}

fn prompt_grain_to_buy(game: &EmpireGame, id: Kingdoms) -> Option<Trade> {
    draw_resource_management_page(game, id);
    let seller = Kingdoms::from_number(bottom_input_number_with_range(
        Some("A quel pays (donner n°) :"),
        1,
        KINGDOMS.len() as i32,
    ))?;
    Some(Trade::Buy {
        amount: bottom_input_number_with_range(Some("Combien de boisseaux achetez-vous ?"), 1, 500),
        seller,
    })
}
