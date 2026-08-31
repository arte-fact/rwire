//! Mobile-first views over the shared [`Room`]. Pure functions: `page` is
//! re-run for every connection whenever the room changes.

use std::borrow::Cow;

use empire_lib::investments::InvestmentType;
use empire_lib::trade::MAX_GRAIN_PRICE;
use empire_lib::{Kingdom, Kingdoms, PlayerTitle, KINGDOMS};
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec, St};
use rwire_components::{
    Alert, Badge, Button, ButtonSize, Card, CardPadding, FormField, Gap, Grid, GridColumns, Input,
    Progress, Select, Spinner, Stack, StackJustify, Stat, Stepper, Table, TableRow, Text,
    TextVariant,
};

use crate::room::{self, by, invest_fr, Battle, Room, Seat, Stage, Step};

type Label = Cow<'static, str>;

/// Bottom tabs.
const TABS: [&str; 3] = ["Partie", "Royaumes", "Journal"];

pub fn page(room: &Room, token: u64, tab: u8) -> ElementBuilder {
    let me = room.seat_of(token);
    let (content, action) = match tab {
        1 => (kingdoms_tab(room), None),
        2 => (journal_tab(room), None),
        _ => partie(room, token, me),
    };
    el(El::Div)
        .st([
            St::MinHDvh,
            St::BgApp,
            St::TextDefault,
            St::DisplayFlex,
            St::FlexCol,
        ])
        .append([
            header(room, me),
            el(El::Main)
                .st([
                    St::Flex1,
                    St::WFull,
                    St::MaxWMd,
                    St::MxAuto,
                    St::PMd,
                    St::DisplayFlex,
                    St::FlexCol,
                    St::GapMd,
                ])
                .append([content, el(El::Div).st([St::MinH6rem])]),
            bottom_bar(action, tab),
        ])
}

// ---------------------------------------------------------------------------
// Chrome
// ---------------------------------------------------------------------------

fn header(room: &Room, me: Option<Kingdoms>) -> ElementBuilder {
    let (title, sub) = match me {
        Some(id) if room.stage != Stage::Lobby => {
            let k = room.game.kingdom(id);
            (
                k.full_title(),
                format!(
                    "{} {} · {} boisseaux · {} hommes d'armes",
                    fmt(k.treasury),
                    k.currency(),
                    fmt(k.grain_stocks),
                    fmt(k.soldiers)
                ),
            )
        }
        Some(id) => (
            room.game.kingdom(id).full_title(),
            "En attente du début de la partie".into(),
        ),
        None => (
            "Empire".to_string(),
            "Six royaumes, un seul empereur.".into(),
        ),
    };
    el(El::Header)
        .st([
            St::PositionSticky,
            St::Top0,
            St::Z10,
            St::BgSurface,
            St::BorderB,
            St::PxMd,
            St::PySm,
        ])
        .append([el(El::Div)
            .st([
                St::MaxWMd,
                St::MxAuto,
                St::DisplayFlex,
                St::ItemsCenter,
                St::JustifyBetween,
                St::GapSm,
            ])
            .append([
                el(El::Div).st([St::MinW0]).append([
                    el(El::Strong)
                        .st([St::DisplayBlock, St::Truncate])
                        .text(&title),
                    Text::caption(sub).muted().build(),
                ]),
                Badge::primary(format!("An {}", room.game.year)).build(),
            ])])
}

/// Fixed bottom bar: the current primary action (if any) above the tabs.
fn bottom_bar(action: Option<ElementBuilder>, tab: u8) -> ElementBuilder {
    let tabs = Stack::row()
        .gap(Gap::Sm)
        .children(TABS.iter().enumerate().map(|(i, label)| {
            let b = if i as u8 == tab {
                Button::secondary(*label)
            } else {
                Button::ghost(*label)
            };
            b.size(ButtonSize::Sm)
                .full_width(true)
                .on_click(crate::set_tab().with_param_bytes(vec![i as u8]))
        }))
        .build();
    let mut rows = Vec::new();
    if let Some(action) = action {
        rows.push(action);
    }
    rows.push(tabs);
    el(El::Div)
        .st([
            St::PositionFixed,
            St::Bottom0,
            St::Left0,
            St::Right0,
            St::Z20,
            St::BgSurface,
            St::BorderT,
            St::PxMd,
            St::PySm,
        ])
        .append([el(El::Div)
            .st([St::MaxWMd, St::MxAuto, St::WFull])
            .append([Stack::column().gap(Gap::Sm).children(rows).build()])])
}

fn journal_tab(room: &Room) -> ElementBuilder {
    if room.log.is_empty() {
        return Text::body("Le journal est encore vierge.").muted().build();
    }
    section(
        "Journal",
        Stack::column()
            .gap(Gap::Xs)
            .children(
                room.log
                    .iter()
                    .rev()
                    .map(|line| Text::body_small(line.clone()).build()),
            )
            .build(),
    )
}

fn kingdoms_tab(room: &Room) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Md)
        .children([
            Grid::new()
                .columns(GridColumns::Fixed2)
                .gap(Gap::Sm)
                .children([
                    Stat::new(fmt(room.game.barbarians_surface))
                        .label("Terres barbares")
                        .build(),
                    Stat::new(room.game.alive_kingdoms().len().to_string())
                        .label("Royaumes en lice")
                        .build(),
                ])
                .build(),
            kingdoms_table(room),
        ])
        .build()
}

// ---------------------------------------------------------------------------
// Partie tab
// ---------------------------------------------------------------------------

type View = (ElementBuilder, Option<ElementBuilder>);

fn partie(room: &Room, token: u64, me: Option<Kingdoms>) -> View {
    match room.stage {
        Stage::Lobby => (
            lobby(room, token, me),
            me.map(|_| primary("Commencer la partie", by(room::start(), token, &[]))),
        ),
        Stage::Over => (
            over(room),
            me.map(|_| primary("Nouvelle partie", by(room::new_game(), token, &[]))),
        ),
        Stage::Playing => {
            if let Some(b) = &room.battle {
                return (battle_page(room, b), None);
            }
            match me {
                Some(id) if room.active() == Some(id) => turn(room, token, id),
                Some(id) if room.game.kingdom(id).is_dead => (
                    Stack::column()
                        .gap(Gap::Md)
                        .children([
                            Alert::error()
                                .title("Votre royaume est tombé")
                                .message("Vous suivez désormais la partie en spectateur.")
                                .build(),
                            waiting(room),
                        ])
                        .build(),
                    None,
                ),
                _ => (waiting(room), None),
            }
        }
    }
}

fn lobby(room: &Room, token: u64, me: Option<Kingdoms>) -> ElementBuilder {
    let mut items = vec![
        Text::new()
            .variant(TextVariant::Heading1)
            .content("E M P I R E")
            .build(),
        Text::body(
            "Choisissez votre royaume. Les royaumes sans seigneur seront joués par l'ordinateur.",
        )
        .muted()
        .build(),
    ];

    for (i, id) in KINGDOMS.into_iter().enumerate() {
        let k = room.game.kingdom(id);
        let (badge, action) = match room.seat(id).owner {
            Some(o) if o == token => (
                Badge::success("Vous").build(),
                Button::secondary("Quitter")
                    .size(ButtonSize::Sm)
                    .on_click(by(room::leave(), token, &[])),
            ),
            Some(_) => (
                Badge::warning(k.player_name.clone()).build(),
                Button::ghost("Pris")
                    .size(ButtonSize::Sm)
                    .disabled(true)
                    .build(),
            ),
            None => (
                Badge::default_badge("Ordinateur").build(),
                Button::primary("Rejoindre")
                    .size(ButtonSize::Sm)
                    .on_click(by(room::join(), token, &[i as u8])),
            ),
        };
        items.push(
            Card::new()
                .padding(CardPadding::Sm)
                .child(
                    Stack::row()
                        .justify(StackJustify::Between)
                        .align_center()
                        .gap(Gap::Sm)
                        .children([
                            el(El::Div).append([
                                el(El::Strong).text(id.name()),
                                Text::caption(format!(
                                    "{} {}",
                                    id.title_name(PlayerTitle::Duke),
                                    id.default_king_name()
                                ))
                                .muted()
                                .build(),
                            ]),
                            Stack::row()
                                .gap(Gap::Sm)
                                .align_center()
                                .children([badge, action])
                                .build(),
                        ])
                        .build(),
                )
                .build(),
        );
    }

    if let Some(id) = me {
        items.push(section(
            "Votre nom",
            form(
                by(room::rename(), token, &[]),
                [
                    Input::text()
                        .name("name")
                        .id("name")
                        .value(room.game.kingdom(id).player_name.clone())
                        .placeholder("Nom du seigneur")
                        .required(true)
                        .build(),
                    Button::secondary("Prendre ce nom").full_width(true).build(),
                ],
            ),
        ));
    }
    items.push(
        Text::caption(format!("{} seigneur(s) à table", room.humans().count()))
            .muted()
            .build(),
    );

    Stack::column().gap(Gap::Md).children(items).build()
}

fn over(room: &Room) -> ElementBuilder {
    let mut standing: Vec<&Kingdom> = room.game.kingdoms.iter().filter(|k| !k.is_dead).collect();
    standing.sort_by_key(|k| std::cmp::Reverse((k.surface, k.total_population())));
    let winner = standing
        .first()
        .map(|k| format!("{} règne sur le plus vaste domaine.", k.full_title()))
        .unwrap_or_else(|| "Tous les royaumes sont tombés.".to_string());
    let mut t = Table::new()
        .headers(["Rang", "Seigneur", "Terres", "Sujets"])
        .striped(true);
    for (i, k) in standing.iter().enumerate() {
        t = t.row(TableRow::new().cells([
            (i + 1).to_string(),
            k.full_title(),
            fmt(k.surface),
            fmt(k.total_population()),
        ]));
    }
    Stack::column()
        .gap(Gap::Md)
        .children([
            Alert::success()
                .title("Fin de la partie")
                .message(winner)
                .build(),
            scroll(t.build()),
        ])
        .build()
}

/// What everyone who is not playing right now sees.
fn waiting(room: &Room) -> ElementBuilder {
    let Some(active) = room.active() else {
        return el(El::Div);
    };
    let a = room.game.kingdom(active);
    let msg = if room.is_computer(active) {
        "L'ordinateur joue…".to_string()
    } else {
        format!("Étape : {}", room.step.label())
    };
    let order = room.game.alive_kingdoms().into_iter().map(|id| {
        let k = room.game.kingdom(id);
        let badge = match id.index().cmp(&room.turn) {
            std::cmp::Ordering::Less => Badge::default_badge("Joué"),
            std::cmp::Ordering::Equal => Badge::primary("En cours"),
            std::cmp::Ordering::Greater => Badge::default_badge("À venir"),
        };
        Stack::row()
            .justify(StackJustify::Between)
            .align_center()
            .children([
                Text::body(format!(
                    "{}{}",
                    k.full_title(),
                    if room.is_computer(id) {
                        " (ordinateur)"
                    } else {
                        ""
                    }
                ))
                .build(),
                badge.build(),
            ])
            .build()
    });
    Stack::column()
        .gap(Gap::Md)
        .children([
            Alert::info()
                .title(format!("Au tour de {}", a.full_title()))
                .message(msg)
                .build(),
            section(
                "Ordre du tour",
                Stack::column().gap(Gap::Sm).children(order).build(),
            ),
            kingdoms_table(room),
        ])
        .build()
}

/// The live battle, replayed identically on every screen.
fn battle_page(room: &Room, b: &Battle) -> ElementBuilder {
    let f = b.frame();
    let a = room.game.kingdom(b.attack.attacker);
    let (defender, count, start, land) = match b.attack.target {
        Some(t) => {
            let d = room.game.kingdom(t);
            if f.population_defending {
                (
                    d.full_title(),
                    f.defender_peasants,
                    d.peasants,
                    Some(t.name()),
                )
            } else {
                (d.full_title(), f.defender_soldiers, b.defender_start, None)
            }
        }
        None => (
            "Barbares païens".to_string(),
            f.defender_soldiers,
            b.defender_start,
            None,
        ),
    };
    let mut items = vec![
        Text::caption("Hommes d'armes restants").muted().build(),
        side(a.full_title(), f.attacker_soldiers, b.attack.soldiers),
        side(defender, count, start),
    ];
    if let Some(name) = land {
        items.push(
            Alert::warning()
                .message(format!("Les serfs de {name} doivent défendre leur pays !"))
                .build(),
        );
    }
    items.push(if b.finished() {
        Alert::info()
            .title("Expédition terminée")
            .message(b.summary.clone())
            .build()
    } else {
        Spinner::new().label("La bataille fait rage…").build()
    });
    section(
        "Bataille",
        Stack::column().gap(Gap::Md).children(items).build(),
    )
}

fn side(name: String, count: i32, start: i32) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Xs)
        .children([
            Stack::row()
                .justify(StackJustify::Between)
                .align_center()
                .children([
                    Text::body(name).build(),
                    el(El::Strong).text(&fmt(count.max(0))),
                ])
                .build(),
            Progress::new()
                .value(count.max(0) as u32)
                .max(start.max(1) as u32)
                .build(),
        ])
        .build()
}

// ---------------------------------------------------------------------------
// The active player's turn
// ---------------------------------------------------------------------------

fn turn(room: &Room, token: u64, id: Kingdoms) -> View {
    let k = room.game.kingdom(id);
    let seat = room.seat(id);
    let mut items = vec![stepper(room.step)];
    if let Some(notice) = &seat.notice {
        items.push(Alert::info().message(notice.clone()).build());
    }
    let (body, action) = match room.step {
        Step::Weather => (weather_step(room), Some(next("Continuer", token))),
        Step::Trade => (
            trade_step(room, id, token),
            Some(next("Passer à l'intendance", token)),
        ),
        Step::Feed => (feed_step(k, token), None),
        Step::Report => (report_step(seat), Some(next("Continuer", token))),
        Step::Economy => (
            economy_step(k, seat, token),
            Some(next("Passer à la guerre", token)),
        ),
        Step::War => (war_step(room, id, token), Some(next("Fin du tour", token))),
    };
    items.push(body);
    (Stack::column().gap(Gap::Md).children(items).build(), action)
}

fn stepper(step: Step) -> ElementBuilder {
    let mut s = Stepper::new();
    for label in Step::LABELS {
        s = s.step(label);
    }
    scroll(s.current(step.index()).build())
}

fn weather_step(room: &Room) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Md)
        .children([
            Alert::info()
                .title(format!("An {}", room.game.year))
                .message(room.game.weather.sentence())
                .build(),
            kingdoms_table(room),
        ])
        .build()
}

fn trade_step(room: &Room, id: Kingdoms, token: u64) -> ElementBuilder {
    let k = room.game.kingdom(id);
    let mut sellers = Select::new().name("seller");
    let mut offers = 0;
    for other in room.game.alive_kingdoms().into_iter().filter(|&o| o != id) {
        let s = room.game.kingdom(other);
        if s.grain_to_sell < 1 || s.grain_price < 1 {
            continue;
        }
        offers += 1;
        sellers = sellers.option(
            (other.index() + 1).to_string(),
            format!(
                "{} — {} boisseaux à {}",
                other.name(),
                fmt(s.grain_to_sell),
                s.grain_price.min(MAX_GRAIN_PRICE)
            ),
        );
    }
    let buy = if offers == 0 {
        Text::body("Personne ne vend de grain cette année.")
            .muted()
            .build()
    } else {
        form(
            by(room::buy_grain(), token, &[]),
            [
                labeled("Vendeur", sellers.build()),
                labeled(
                    "Boisseaux (max 500, courtage 10 %)",
                    number("buy_amount", 1, 500, 100),
                ),
                Button::secondary("Acheter").full_width(true).build(),
            ],
        )
    };
    Stack::column()
        .gap(Gap::Md)
        .children([
            resources(k),
            section("Acheter du grain", buy),
            section(
                "Vendre du grain",
                form(
                    by(room::sell_grain(), token, &[]),
                    [
                        labeled(
                            "Boisseaux",
                            number(
                                "sell_amount",
                                1,
                                k.grain_stocks.max(1),
                                (k.grain_stocks / 10).max(1),
                            ),
                        ),
                        labeled(
                            "Prix du boisseau (max 15)",
                            number("price", 1, MAX_GRAIN_PRICE, 5),
                        ),
                        Button::secondary("Mettre en vente")
                            .full_width(true)
                            .build(),
                    ],
                ),
            ),
            section(
                "Vendre des terres",
                form(
                    by(room::sell_land(), token, &[]),
                    [
                        labeled(
                            "Arpents (2 pièces l'arpent)",
                            number("arpents", 1, (k.surface - 1).max(1), 100),
                        ),
                        Button::secondary("Vendre aux Barbares")
                            .full_width(true)
                            .build(),
                    ],
                ),
            ),
        ])
        .build()
}

fn feed_step(k: &Kingdom, token: u64) -> ElementBuilder {
    let stocks = k.grain_stocks.max(0);
    let peasants = k.peasants_grain_needs().min(stocks);
    let soldiers = k.soldiers_grain_needs().min(stocks - peasants);
    Stack::column()
        .gap(Gap::Md)
        .children([
            resources(k),
            section(
                "Nourrir le royaume",
                form(
                    by(room::feed(), token, &[]),
                    [
                        labeled(
                            format!(
                                "Grain pour les {} habitants (besoin : {})",
                                fmt(k.population()),
                                fmt(k.peasants_grain_needs())
                            ),
                            number("peasants", 0, stocks, peasants),
                        ),
                        labeled(
                            format!(
                                "Grain pour l'ost de {} hommes (besoin : {})",
                                fmt(k.soldiers),
                                fmt(k.soldiers_grain_needs())
                            ),
                            number("soldiers", 0, stocks, soldiers),
                        ),
                        Text::caption(
                            "Mal nourris, serfs et soldats meurent ou désertent ; bien nourris, les étrangers immigrent.",
                        )
                        .muted()
                        .build(),
                        Button::primary("Nourrir")
                            .size(ButtonSize::Lg)
                            .full_width(true)
                            .build(),
                    ],
                ),
            ),
        ])
        .build()
}

fn report_step(seat: &Seat) -> ElementBuilder {
    let Some(d) = &seat.demo else {
        return el(El::Div);
    };
    let lines = [
        (d.births, "naissances"),
        (d.disease_victims, "habitants morts de maladie"),
        (d.malnutrition_victims, "habitants morts de faim"),
        (d.starvation_victims, "habitants morts de misère"),
        (d.immigrants, "étrangers ont immigré dans votre pays"),
        (
            d.soldiers_starvation_victims,
            "hommes d'armes morts d'épuisement",
        ),
        (d.soldiers_desertion_victims, "hommes d'armes ont déserté"),
    ];
    let mut items: Vec<ElementBuilder> = lines
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, label)| Text::body(format!("{} {}", fmt(*n), label)).build())
        .collect();
    if items.is_empty() {
        items.push(Text::body("Une année sans histoire.").muted().build());
    }
    items.push(
        Text::body(format!(
            "Votre ost combattra avec une efficacité de {}0 %.",
            d.soldiers_efficiency
        ))
        .build(),
    );
    let losses = d.disease_victims
        + d.malnutrition_victims
        + d.starvation_victims
        + d.soldiers_starvation_victims
        + d.soldiers_desertion_victims;
    let delta = d.births + d.immigrants - losses;
    let verb = match delta {
        n if n > 0 => "gagné",
        n if n < 0 => "perdu",
        _ => "conservé",
    };
    items.push(
        Text::body(format!(
            "Vous avez {verb} {} sujets taillables et corvéables à merci.",
            fmt(delta.abs())
        ))
        .build(),
    );
    section(
        "Le peuple",
        Stack::column().gap(Gap::Xs).children(items).build(),
    )
}

fn economy_step(k: &Kingdom, seat: &Seat, token: u64) -> ElementBuilder {
    let mut items = Vec::new();

    if let Some(e) = &seat.eco {
        let rows = [
            ("Champs de foire", k.marketplaces, e.marketplaces_profits),
            ("Moulins à grain", k.grain_mills, e.grain_mills_profits),
            ("Fonderies", k.foundries, e.foundries_profits),
            ("Chantiers navals", k.shipyards, e.shipyards_profits),
            ("Hommes d'armes", k.soldiers, -e.soldiers_maintenance),
            (
                "Droits de douane (%)",
                k.immigration_taxes,
                e.immigration_taxes_profits,
            ),
            (
                "Taxe commerciale (%)",
                k.commercial_taxes,
                e.commercial_taxes_profits,
            ),
            ("Impôts directs (%)", k.income_taxes, e.income_taxes_profits),
        ];
        let mut t = Table::new()
            .headers(["Poste", "Nombre", "Profits"])
            .striped(true);
        for (label, count, profit) in rows {
            t = t.row(TableRow::new().cells([label.to_string(), fmt(count), fmt(profit)]));
        }
        items.push(section(
            format!("Revenus d'état : {} {}", fmt(e.net()), k.currency()),
            scroll(t.build()),
        ));
    }

    items.push(section(
        "Taux d'imposition",
        form(
            by(room::set_taxes(), token, &[]),
            [
                labeled(
                    "Droits de douane (max 50 %)",
                    number("customs", 0, 50, k.immigration_taxes),
                ),
                labeled(
                    "Taxe commerciale (max 20 %)",
                    number("sales", 0, 20, k.commercial_taxes),
                ),
                labeled(
                    "Impôts directs (max 35 %)",
                    number("income", 0, 35, k.income_taxes),
                ),
                Button::secondary("Promulguer").full_width(true).build(),
            ],
        ),
    ));

    let mut kinds = Select::new().name("kind");
    for (n, kind) in (1..=6).filter_map(|n| InvestmentType::from_number(n).map(|k| (n, k))) {
        kinds = kinds.option(
            n.to_string(),
            format!(
                "{} — {} {} pièce (max {})",
                invest_fr(kind),
                fmt(kind.cost()),
                k.currency(),
                kind.max_investment(k).max(0)
            ),
        );
    }
    items.push(section(
        format!(
            "Investissements — trésor : {} {}",
            fmt(k.treasury),
            k.currency()
        ),
        form(
            by(room::invest(), token, &[]),
            [
                labeled("Type", kinds.build()),
                labeled("Quantité", number("invest_amount", 0, 99_999, 1)),
                Button::secondary("Investir").full_width(true).build(),
            ],
        ),
    ));

    Stack::column().gap(Gap::Md).children(items).build()
}

fn war_step(room: &Room, id: Kingdoms, token: u64) -> ElementBuilder {
    let k = room.game.kingdom(id);
    let year = room.game.year;
    let mut t = Table::new()
        .headers(["Terres vassales", "Arpents", "Soldats"])
        .striped(true)
        .row(TableRow::new().cells([
            "Barbares".to_string(),
            fmt(room.game.barbarians_surface),
            "?".to_string(),
        ]));
    let mut targets = Select::new().name("target").option("0", "Barbares");
    for other in room.game.alive_kingdoms().into_iter().filter(|&o| o != id) {
        let o = room.game.kingdom(other);
        t = t.row(TableRow::new().cells([o.full_title(), fmt(o.surface), fmt(o.soldiers)]));
        if year >= 3 {
            targets = targets.option((other.index() + 1).to_string(), o.full_title());
        }
    }
    let hint = if year < 3 {
        "Les autres royaumes ne peuvent être attaqués qu'à partir de la 3ème année."
    } else {
        "Conquérir toutes les terres d'un royaume l'annexe : ses serfs deviennent les vôtres."
    };
    Stack::column()
        .gap(Gap::Md)
        .children([
            scroll(t.build()),
            section(
                "Expédition",
                form(
                    by(room::attack(), token, &[]),
                    [
                        labeled("Cible", targets.build()),
                        labeled(
                            format!("Hommes d'armes (vous en avez {})", fmt(k.soldiers)),
                            number("soldiers", 1, k.soldiers.max(1), (k.soldiers / 2).max(1)),
                        ),
                        Text::caption(hint).muted().build(),
                        Button::destructive("Attaquer").full_width(true).build(),
                    ],
                ),
            ),
        ])
        .build()
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

fn kingdoms_table(room: &Room) -> ElementBuilder {
    let mut t = Table::new()
        .headers([
            "Royaume",
            "Nobles",
            "Soldats",
            "Marchands",
            "Serfs",
            "Terres",
            "Palais",
        ])
        .striped(true);
    for id in room.game.alive_kingdoms() {
        let k = room.game.kingdom(id);
        t = t.row(TableRow::new().cells([
            format!("{} ({})", k.name(), k.player_name),
            fmt(k.nobles),
            fmt(k.soldiers),
            fmt(k.merchants),
            fmt(k.peasants),
            fmt(k.surface),
            format!("{} %", k.palaces * 10),
        ]));
    }
    scroll(t.build())
}

fn resources(k: &Kingdom) -> ElementBuilder {
    Grid::new()
        .columns(GridColumns::Fixed2)
        .gap(Gap::Sm)
        .children([
            Stat::new(fmt(k.grain_harvest)).label("Récolte").build(),
            Stat::new(fmt(k.grain_stocks))
                .label("Réserves de grain")
                .build(),
            Stat::new(fmt(k.peasants_grain_needs()))
                .label("Besoins du peuple")
                .build(),
            Stat::new(fmt(k.soldiers_grain_needs()))
                .label("Besoins de l'ost")
                .build(),
            Stat::new(fmt(k.treasury))
                .label(format!("Trésor ({})", k.currency()))
                .build(),
            Stat::new(format!("{} %", k.rats_loss_rate))
                .label("Mangé par les rats")
                .build(),
        ])
        .build()
}

fn section(title: impl Into<Label>, body: ElementBuilder) -> ElementBuilder {
    Card::new()
        .padding(CardPadding::Md)
        .children([
            Text::new()
                .variant(TextVariant::Heading3)
                .content(title)
                .build(),
            body,
        ])
        .build()
}

fn scroll(inner: ElementBuilder) -> ElementBuilder {
    el(El::Div)
        .st([St::OverflowXAuto, St::WFull, St::WhitespaceNowrap])
        .append([inner])
}

fn form(
    on_submit: HandlerSpec,
    children: impl IntoIterator<Item = ElementBuilder>,
) -> ElementBuilder {
    el(El::Form)
        .st([St::DisplayFlex, St::FlexCol, St::GapSm])
        .on(Ev::Submit, on_submit)
        .append(children)
}

fn labeled(label: impl Into<Label>, input: ElementBuilder) -> ElementBuilder {
    FormField::new().label(label).input(input).build()
}

fn number(name: &'static str, min: i32, max: i32, value: i32) -> ElementBuilder {
    Input::number()
        .name(name)
        .id(name)
        .min(min.to_string())
        .max(max.to_string())
        .value(value.clamp(min, max).to_string())
        .required(true)
        .build()
}

fn primary(label: &'static str, spec: HandlerSpec) -> ElementBuilder {
    Button::primary(label)
        .size(ButtonSize::Lg)
        .full_width(true)
        .on_click(spec)
}

fn next(label: &'static str, token: u64) -> ElementBuilder {
    primary(label, by(room::advance(), token, &[]))
}

/// 12345 → "12 345"
fn fmt(n: i32) -> String {
    let digits: Vec<char> = n.abs().to_string().chars().collect();
    let mut out = String::new();
    for (i, c) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{202F}');
        }
        out.push(*c);
    }
    if n < 0 {
        out.insert(0, '-');
    }
    out
}
