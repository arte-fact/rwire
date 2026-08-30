use empire_lib::economy::{apply_tax_change, TaxType, YearEconomy};
use empire_lib::investments::{apply_investment, InvestmentType};
use empire_lib::Kingdom;

use crate::ui::{
    black_on_gray, bottom_input_number_with_range, bottom_input_number_with_range_or_enter,
    clear_screen, large_number, print_at,
};

fn display_economic_report(kingdom: &Kingdom, year: i32, eco: &YearEconomy) {
    clear_screen();

    print_at(0, 28, &kingdom.full_title());
    print_at(1, 34, &format!("en l'an {} ", year));
    print_at(2, 3, &black_on_gray(" Revenus d'état "));
    print_at(
        3,
        12,
        &format!(
            " Trésor =   {:>11} {}",
            large_number(kingdom.treasury),
            kingdom.currency()
        ),
    );
    print_at(
        4,
        13,
        &black_on_gray("  Droits         Taxe            Impôts  "),
    );
    print_at(
        5,
        13,
        &black_on_gray(" de douane     commerciale       directs "),
    );
    print_at(
        6,
        12,
        &format!(
            "    {:>2} %           {:>2} %            {:>2} %  ",
            kingdom.immigration_taxes, kingdom.commercial_taxes, kingdom.income_taxes
        ),
    );
    print_at(
        7,
        12,
        &format!(
            "{:>8}       {:>8}        {:>8}  ",
            large_number(eco.immigration_taxes_profits),
            large_number(eco.commercial_taxes_profits),
            large_number(eco.income_taxes_profits)
        ),
    );
    print_at(
        9,
        5,
        &black_on_gray(" Investissements        Nombre         Profits        Coûts "),
    );

    let rows = [
        (
            "1) Champs de foire  ",
            kingdom.marketplaces,
            eco.marketplaces_profits,
            InvestmentType::Marketplaces,
        ),
        (
            "2) Moulins à grain  ",
            kingdom.grain_mills,
            eco.grain_mills_profits,
            InvestmentType::GrainMills,
        ),
        (
            "3) Fonderies        ",
            kingdom.foundries,
            eco.foundries_profits,
            InvestmentType::Foundries,
        ),
        (
            "4) Chantiers navals ",
            kingdom.shipyards,
            eco.shipyards_profits,
            InvestmentType::Shipyards,
        ),
        (
            "5) Hommes d'armes   ",
            kingdom.soldiers,
            -eco.soldiers_maintenance,
            InvestmentType::Soldiers,
        ),
    ];
    for (i, (label, count, profit, kind)) in rows.iter().enumerate() {
        print_at(
            10 + i,
            3,
            &format!(
                "{}   {:>8}        {:>8}      {:>8}",
                label,
                count,
                large_number(*profit),
                kind.cost()
            ),
        );
    }
    print_at(
        15,
        3,
        &format!(
            "6) Palais              {:>8}% terminé             {:>8}",
            large_number(kingdom.palaces * 10),
            InvestmentType::Palaces.cost()
        ),
    );
}

fn prompt_tax_option(kingdom: &Kingdom, year: i32, eco: &YearEconomy) -> Option<TaxType> {
    display_economic_report(kingdom, year, eco);
    print_at(
        20,
        0,
        "1/Droits de douane,2/Taxe commerciale,3/Impôts directs ou ↵/Investissement ?",
    );
    bottom_input_number_with_range_or_enter(None, 1, 3).and_then(TaxType::from_number)
}

fn prompt_tax_amount(tax_type: TaxType, kingdom: &Kingdom, year: i32, eco: &YearEconomy) -> i32 {
    display_economic_report(kingdom, year, eco);
    let max = tax_type.max_rate();
    let text = match tax_type {
        TaxType::Immigration => format!("Nouveaux droits de douane (max={:>2}%) ?", max),
        TaxType::Commercial => format!("Nouvelles taxes commerciales (max={:>2}%) ?", max),
        TaxType::Income => format!("Nouveaux impôts directs (max={:>2}%) ?", max),
    };
    print_at(20, 0, &text);
    bottom_input_number_with_range(None, 0, max)
}

pub fn taxes_management(kingdom: &mut Kingdom, year: i32, eco: &YearEconomy) {
    while let Some(tax_type) = prompt_tax_option(kingdom, year, eco) {
        let rate = prompt_tax_amount(tax_type, kingdom, year, eco);
        apply_tax_change(kingdom, tax_type, rate);
    }
}

fn prompt_investment_option(
    kingdom: &Kingdom,
    year: i32,
    eco: &YearEconomy,
    error: Option<&str>,
) -> Option<InvestmentType> {
    display_economic_report(kingdom, year, eco);
    print_at(20, 0, "↵ ou autre investissement (donner n°) :");
    bottom_input_number_with_range_or_enter(error, 1, 6).and_then(InvestmentType::from_number)
}

fn prompt_investment_amount(
    investment_type: InvestmentType,
    kingdom: &Kingdom,
    year: i32,
    eco: &YearEconomy,
) -> i32 {
    display_economic_report(kingdom, year, eco);
    let max = investment_type.max_investment(kingdom);
    print_at(
        20,
        0,
        &format!("Montant de l'investissement ? (max={}) ", max),
    );
    bottom_input_number_with_range(None, 0, max.max(0))
}

pub fn investments_management(kingdom: &mut Kingdom, year: i32, eco: &YearEconomy) {
    let mut error: Option<String> = None;
    while let Some(investment_type) = prompt_investment_option(kingdom, year, eco, error.as_deref())
    {
        let amount = prompt_investment_amount(investment_type, kingdom, year, eco);
        let result = apply_investment(kingdom, investment_type, amount);
        error = if result.success { None } else { result.error };
    }
}
