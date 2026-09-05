use std::thread::sleep;
use std::time::Duration;

use empire_lib::war::{
    apply_barbarian_battle_result, apply_kingdom_battle_result, simulate_barbarian_battle,
    simulate_kingdom_battle, BarbarianBattleResult, BattleProgress, BattleResult, CollateralDamage,
};
use empire_lib::{EmpireGame, Kingdoms};

use crate::ui::{
    black_on_gray, bottom_input_number_with_range_or_enter, bottom_press_enter, clear_screen,
    large_number, print_at,
};

const AUTO_CLOSE_DELAY: u64 = 2500;

enum AttackDefender {
    Kingdom(Kingdoms),
    Barbarians,
}

struct Attack {
    defender: AttackDefender,
    soldiers: i32,
}

pub fn war_management(game: &mut EmpireGame, id: Kingdoms) {
    while let Some(attack) = prompt_kingdom_to_attack(game, id) {
        match attack.defender {
            AttackDefender::Kingdom(defender) => {
                attack_kingdom(game, id, defender, attack.soldiers, false)
            }
            AttackDefender::Barbarians => attack_barbarians(game, id, attack.soldiers, false),
        }
    }
}

pub fn attack_kingdom(
    game: &mut EmpireGame,
    attacker: Kingdoms,
    defender: Kingdoms,
    soldiers: i32,
    auto_close: bool,
) {
    let attacker_name = game.kingdom(attacker).full_title();
    let defender_name = game.kingdom(defender).full_title();

    let result = simulate_kingdom_battle(game, attacker, defender, soldiers, |progress| {
        print_war_page(&WarPageData {
            attacker_name: &attacker_name,
            attacker_soldiers: progress.attacker_soldiers,
            defender_name: &defender_name,
            defender_land_name: defender.name(),
            defender_soldiers: progress.defender_soldiers,
            defender_peasants: progress.defender_peasants,
            population_defending: progress.population_defending,
        });
        sleep(Duration::from_millis(100));
    });

    apply_kingdom_battle_result(game, attacker, defender, soldiers, &result);

    if result.defender_conquered {
        land_conquered(&defender_name, auto_close);
    } else {
        battle_over(&attacker_name, &result, auto_close);
    }
}

pub fn attack_barbarians(
    game: &mut EmpireGame,
    attacker: Kingdoms,
    soldiers: i32,
    auto_close: bool,
) {
    let attacker_name = game.kingdom(attacker).full_title();
    let attacker_title = game.kingdom(attacker).titled_name();

    let result =
        simulate_barbarian_battle(game, attacker, soldiers, |progress: &BattleProgress| {
            print_war_page(&WarPageData {
                attacker_name: &attacker_name,
                attacker_soldiers: progress.attacker_soldiers,
                defender_name: "Barbares païens",
                defender_land_name: "Barbares",
                defender_soldiers: progress.defender_soldiers,
                defender_peasants: 0,
                population_defending: false,
            });
            sleep(Duration::from_millis(100));
        });

    apply_barbarian_battle_result(game, attacker, soldiers, &result);
    display_barbarian_battle_result(&attacker_title, &result, auto_close);
}

fn wait(auto_close: bool, millis: u64) {
    if auto_close {
        sleep(Duration::from_millis(millis));
    } else {
        bottom_press_enter();
    }
}

fn display_barbarian_battle_result(
    attacker_title: &str,
    result: &BarbarianBattleResult,
    auto_close: bool,
) {
    clear_screen();
    print_at(5, 20, "Expédition terminée");

    if result.attacker_won {
        print_at(7, 16, &format!("{} gagne.", attacker_title));
        print_at(
            8,
            11,
            &format!("{} arpents conquis.", result.surface_conquered),
        );
    } else {
        print_at(7, 15, &format!("{} perd.", attacker_title));
        print_at(
            8,
            1,
            &format!(
                "Malgré cette défaite vous arrivez quand même a conquérir {} arpents",
                result.surface_conquered
            ),
        );
    }

    wait(auto_close, AUTO_CLOSE_DELAY);
}

fn land_conquered(land_name: &str, auto_close: bool) {
    clear_screen();
    print_at(5, 30, "Expédition terminée");
    print_at(7, 15, &format!("Le pays de {} est conquis!", land_name));
    print_at(9, 15, "Tous les nobles ennemis ont été exterminés.");
    print_at(
        12,
        3,
        "Tous les serfs ennemis vous ont juré fidélité et vous devez maintenant",
    );
    print_at(13, 0, "les considérer comme étant votre peuple.");
    print_at(
        15,
        3,
        "Tous les marchands ennemis ont fui le pays. Malheureusement, tous leurs",
    );
    print_at(
        16,
        0,
        "comptoirs ont été pillés et détruits par votre armée ivre de rage",
    );
    print_at(17, 0, "et de vin qui fêtait la victoire.");
    wait(auto_close, AUTO_CLOSE_DELAY);
}

fn battle_over(attacker_title_name: &str, result: &BattleResult, auto_close: bool) {
    clear_screen();
    print_at(5, 30, "Expédition terminée");

    if result.attacker_won {
        print_at(7, 15, &format!("{} gagne.", attacker_title_name));
        print_at(
            8,
            0,
            &format!("{} arpents conquis.", result.surface_conquered),
        );
    } else {
        print_at(7, 15, &format!("{} perd.", attacker_title_name));
        print_at(
            8,
            0,
            &format!(
                "Malgré cette défaite vous arrivez quand même a conquérir {} arpents",
                result.surface_conquered
            ),
        );
    }

    if let Some(damage) = &result.collateral_damage {
        display_collateral_damage(damage);
    }

    wait(auto_close, 5000);
}

fn display_collateral_damage(damage: &CollateralDamage) {
    let lines = [
        format!(
            "{} serfs ennemis ont été battus et massacrés par vos troupes!",
            damage.peasants_killed
        ),
        format!(
            "{} foires ennemies ont été anéanties",
            damage.marketplaces_destroyed
        ),
        format!(
            "{} boisseaux de grain ennemis ont été brûlés",
            damage.grain_destroyed
        ),
        format!(
            "{} moulins à grain ennemis ont été incendiés",
            damage.grain_mills_destroyed
        ),
        format!(
            "{} fonderies ennemies ont été rasées",
            damage.foundries_destroyed
        ),
        format!(
            "{} chantiers navals ennemis ont été détruits",
            damage.shipyards_destroyed
        ),
        format!("{} nobles ennemis ont été égorgés", damage.nobles_killed),
    ];
    for (i, line) in lines.iter().enumerate() {
        print_at(9 + i, 15, line);
    }
}

fn prompt_kingdom_to_attack(game: &EmpireGame, id: Kingdoms) -> Option<Attack> {
    print_page(game);
    print_at(20, 16, "↵ ou ennemi à attaquer (donner n°) : ");

    let alive = game.alive_kingdoms();
    loop {
        let number = bottom_input_number_with_range_or_enter(None, 1, alive.len() as i32 + 1)?;
        let soldiers = game.kingdom(id).soldiers;
        if soldiers == 0 {
            print_at(20, 0, "Vous n'avez plus d'hommes d'armes.");
            bottom_press_enter();
            return None;
        }

        if number == 1 {
            return Some(Attack {
                soldiers: prompt_number_of_soldiers_to_send(game, soldiers),
                defender: AttackDefender::Barbarians,
            });
        }

        // Menu numbers 2.. map onto the alive kingdoms listed by print_page.
        let Some(&target) = alive.get(number as usize - 2) else {
            print_at(20, 16, "Ce joueur n'existe pas.");
            continue;
        };
        if game.year < 3 {
            print_at(
                20,
                16,
                "Vous ne pouvez pas attaquer les autres joueurs avant la 3ème année.",
            );
            continue;
        }
        if target == id {
            print_at(20, 16, "Vous ne pouvez pas vous attaquer vous-même.");
            continue;
        }

        return Some(Attack {
            soldiers: prompt_number_of_soldiers_to_send(game, soldiers),
            defender: AttackDefender::Kingdom(target),
        });
    }
}

fn prompt_number_of_soldiers_to_send(game: &EmpireGame, max: i32) -> i32 {
    loop {
        print_page(game);
        print_at(20, 0, "Combien d'hommes d'armes envoyez-vous ? ");
        if let Some(number) = bottom_input_number_with_range_or_enter(None, 1, max) {
            return number;
        }
    }
}

struct WarPageData<'a> {
    attacker_name: &'a str,
    defender_name: &'a str,
    defender_land_name: &'a str,
    attacker_soldiers: i32,
    defender_soldiers: i32,
    defender_peasants: i32,
    population_defending: bool,
}

fn print_war_page(war: &WarPageData) {
    clear_screen();
    print_at(5, 35, " Hommes d'armes restants ");
    print_at(7, 10, &format!("{} : ", war.attacker_name));
    print_at(7, 40, &format!("{:>6}", war.attacker_soldiers));
    print_at(8, 10, &format!("{} : ", war.defender_name));
    if war.population_defending {
        print_at(8, 40, &format!("{:>6}", war.defender_peasants));
        print_at(
            10,
            12,
            &format!(
                "Les serfs de {} doivent defendre leur pays!",
                war.defender_land_name
            ),
        );
    } else {
        print_at(8, 40, &format!("{:>6}", war.defender_soldiers));
    }
}

fn print_page(game: &EmpireGame) {
    clear_screen();
    print_at(4, 16, &black_on_gray(" Terres vassales "));
    print_at(6, 16, "1) Barbares");
    print_at(6, 40, "sans fin");

    for (index, id) in game.alive_kingdoms().into_iter().enumerate() {
        print_at(7 + index, 16, &format!("{}) {}", index + 2, id.name()));
        print_at(
            7 + index,
            40,
            &format!("{:>6}", large_number(game.kingdom(id).surface)),
        );
    }
}
