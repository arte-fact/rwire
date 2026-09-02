use empire_lib::demography::YearDemography;
use empire_lib::Kingdom;

use crate::ui::{bottom_press_enter, clear_screen, print_at};

pub fn display_demographic_report(kingdom: &Kingdom, year: i32, report: &YearDemography) {
    clear_screen();

    print_at(6, 28, &kingdom.full_title());
    print_at(7, 34, &format!("en l'an {} ", year));

    let lines = [
        (report.births, "naissances"),
        (report.disease_victims, "habitants morts de maladie"),
        (report.malnutrition_victims, "habitants morts de faim"),
        (report.starvation_victims, "habitant morts de misère"),
        (report.immigrants, "etrangers ont immigré dans votre pays"),
        (
            report.nobles_immigrants,
            "nobles parmi eux ont rejoint la cour",
        ),
        (report.merchants_settled, "marchands ont ouvert boutique"),
        (report.nobles_departed, "nobles ont fui la disette"),
        (report.merchants_departed, "marchands ont fermé boutique"),
        (
            report.soldiers_starvation_victims,
            "homme d'arme morts d'epuisement",
        ),
        (
            report.soldiers_desertion_victims,
            "homme d'arme ont déserté",
        ),
    ];

    let mut row = 9;
    for (count, label) in lines {
        if count > 0 {
            print_at(row, 15, &format!("{:>5} {}", count, label));
            row += 1;
        }
    }

    row += 1;
    print_at(
        row,
        15,
        &format!(
            "Votre ost combattra avec un efficacité de {}%",
            report.soldiers_efficiency
        ),
    );
    row += 1;

    let delta = report.population_delta()
        - report.soldiers_starvation_victims
        - report.soldiers_desertion_victims;
    let verb = match delta {
        d if d > 0 => "gagné",
        d if d < 0 => "perdu",
        _ => "conservé",
    };
    print_at(
        row,
        15,
        &format!(
            "Vous avez {} {} sujets taillables et corvéables à merci",
            verb,
            delta.abs()
        ),
    );

    bottom_press_enter();
}
