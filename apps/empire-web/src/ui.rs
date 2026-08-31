//! Mobile-first views over the shared [`Rooms`]. Pure functions: `page` is
//! re-run for every connection whenever any table changes.

use std::borrow::Cow;

use empire_lib::investments::InvestmentType;
use empire_lib::trade::MAX_GRAIN_PRICE;
use empire_lib::{Kingdom, Kingdoms, PlayerTitle, KINGDOMS};
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec, St};
use rwire_components::{
    Alert, Badge, Button, ButtonSize, Card, CardPadding, CopyButton, FormField, Gap, Grid,
    GridColumns, Input, Link, Progress, Select, Slider, Spinner, Stack, StackJustify, Stat,
    Stepper, Table, TableRow, Text, TextVariant,
};

use crate::room::{self, by, invest_fr, Battle, Room, Rooms, Seat, Stage, Step};

type Label = Cow<'static, str>;

/// Bottom tabs.
const TABS: [&str; 3] = ["Partie", "Royaumes", "Journal"];

/// The viewer's handle on one table: everything a view needs to bind actions.
#[derive(Clone, Copy)]
struct T<'a> {
    room: &'a Room,
    code: &'a str,
    token: u64,
}

impl T<'_> {
    fn act(&self, spec: HandlerSpec) -> HandlerSpec {
        by(spec, self.token, self.code, &[])
    }
}

pub fn page(rooms: &Rooms, token: u64, tab: u8, code: Option<&str>) -> ElementBuilder {
    match code.and_then(|c| rooms.get(c).map(|r| (c, r))) {
        Some((code, room)) => room_page(T { room, code, token }, tab),
        None => home_page(rooms, token, code.is_some()),
    }
}

/// App shell: the root is exactly one dynamic viewport tall and never scrolls;
/// `main` is the scroll container and the bar sits in normal flow below it. No
/// `position: fixed`, so a collapsing mobile address bar can't hide or jolt it.
fn shell(header: ElementBuilder, content: ElementBuilder, bar: ElementBuilder) -> ElementBuilder {
    el(El::Div)
        .st([
            St::HDvh,
            St::BgApp,
            St::TextDefault,
            St::DisplayFlex,
            St::FlexCol,
            St::OverflowHidden,
        ])
        .append([
            header,
            el(El::Main)
                .st([St::Flex1, St::MinH0, St::OverflowYAuto, St::WFull])
                .append([el(El::Div)
                    .st([
                        St::MaxWMd,
                        St::MxAuto,
                        St::PMd,
                        St::DisplayFlex,
                        St::FlexCol,
                        St::GapMd,
                    ])
                    .append([content])]),
            bar,
        ])
}

// ---------------------------------------------------------------------------
// Home: create a table or join one
// ---------------------------------------------------------------------------

fn home_page(rooms: &Rooms, token: u64, unknown: bool) -> ElementBuilder {
    let mut items = vec![
        Text::new()
            .variant(TextVariant::Heading1)
            .content("E M P I R E")
            .build(),
        Text::body("Six royaumes, un seul empereur. Créez une table et partagez son lien avec vos amis ; les royaumes sans seigneur seront joués par l'ordinateur.")
            .muted()
            .build(),
    ];
    if unknown {
        items.push(
            Alert::error()
                .title("Table introuvable")
                .message("Cette table n'existe plus ou le code est erroné.")
                .build(),
        );
    }
    items.push(section(
        "Rejoindre avec un code",
        form(
            by(room::enter_code(), token, "", &[]),
            [
                Input::text()
                    .name("code")
                    .id("code")
                    .placeholder("Code de la table (ex. K7PQ2)")
                    .autocomplete("off")
                    .spellcheck(false)
                    .required(true)
                    .build(),
                Button::secondary("Rejoindre").full_width(true).build(),
            ],
        ),
    ));
    let mine: Vec<ElementBuilder> = rooms
        .mine(token)
        .map(|r| {
            let status = match r.stage {
                Stage::Lobby => format!("{} seigneur(s) à table", r.humans().count()),
                Stage::Playing => format!("An {}", r.game.year),
                Stage::Over => "terminée".to_string(),
            };
            Stack::row()
                .justify(StackJustify::Between)
                .align_center()
                .children([
                    Link::new(format!("/r/{}", r.code))
                        .text(format!("Table {}", r.code))
                        .build(),
                    Text::caption(status).muted().build(),
                ])
                .build()
        })
        .collect();
    if !mine.is_empty() {
        items.push(section(
            "Vos tables",
            Stack::column().gap(Gap::Sm).children(mine).build(),
        ));
    }
    shell(
        header_bar("Empire", "Six royaumes, un seul empereur.", None, None),
        Stack::column().gap(Gap::Md).children(items).build(),
        bottom_bar(
            Some(primary(
                "Créer une table",
                by(room::create_room(), token, "", &[]),
            )),
            None,
        ),
    )
}

// ---------------------------------------------------------------------------
// A table
// ---------------------------------------------------------------------------

fn room_page(t: T, tab: u8) -> ElementBuilder {
    let me = t.room.seat_of(t.token);
    let (content, action) = match tab {
        1 => (kingdoms_tab(t.room), None),
        2 => (journal_tab(t.room), None),
        _ => partie(t, me),
    };
    shell(header(t, me), content, bottom_bar(action, Some(tab)))
}

fn header(t: T, me: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
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
            format!("Table {} · en attente du début", t.code),
        ),
        None => (format!("Table {}", t.code), "Spectateur".to_string()),
    };
    header_bar(
        &title,
        &sub,
        Some(Badge::primary(format!("An {}", room.game.year)).build()),
        Some(
            Button::ghost("Accueil")
                .size(ButtonSize::Sm)
                .on_click(crate::go_home()),
        ),
    )
}

fn header_bar(
    title: &str,
    sub: &str,
    badge: Option<ElementBuilder>,
    action: Option<ElementBuilder>,
) -> ElementBuilder {
    let mut right = Vec::new();
    if let Some(b) = badge {
        right.push(b);
    }
    if let Some(a) = action {
        right.push(a);
    }
    el(El::Header)
        .st([
            St::FlexShrink0,
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
                        .text(title),
                    Text::caption(sub.to_string()).muted().build(),
                ]),
                el(El::Div)
                    .st([St::FlexShrink0, St::WhitespaceNowrap])
                    .append([Stack::row()
                        .gap(Gap::Sm)
                        .align_center()
                        .children(right)
                        .build()]),
            ])])
}

/// Bottom bar: the current primary action (if any) above the tabs.
fn bottom_bar(action: Option<ElementBuilder>, tab: Option<u8>) -> ElementBuilder {
    let mut rows = Vec::new();
    if let Some(action) = action {
        rows.push(action);
    }
    if let Some(tab) = tab {
        rows.push(
            Stack::row()
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
                .build(),
        );
    }
    el(El::Div)
        .st([
            St::FlexShrink0,
            St::BgSurface,
            St::BorderT,
            St::PxMd,
            St::PySm,
            St::PbSafe,
        ])
        .append([el(El::Div)
            .st([St::MaxWMd, St::MxAuto, St::WFull])
            .append([Stack::column().gap(Gap::Sm).children(rows).build()])])
}

fn journal_tab(room: &Room) -> ElementBuilder {
    if room.log.is_empty() {
        return Text::body("Le journal est encore vierge.").muted().build();
    }
    // Chronological, in a scroll box that follows the latest entry.
    section(
        "Journal",
        el(El::Div)
            .st([St::OverflowYAuto, St::MaxH96])
            .data("autoscroll", "1")
            .append([Stack::column()
                .gap(Gap::Xs)
                .children(
                    room.log
                        .iter()
                        .map(|line| Text::body_small(line.clone()).build()),
                )
                .build()]),
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

fn partie(t: T, me: Option<Kingdoms>) -> View {
    let room = t.room;
    match room.stage {
        Stage::Lobby => (
            lobby(t, me),
            me.map(|_| primary("Commencer la partie", t.act(room::start()))),
        ),
        Stage::Over => (
            over(room),
            me.map(|_| primary("Nouvelle partie", t.act(room::new_game()))),
        ),
        Stage::Playing => {
            if let Some(b) = &room.battle {
                return (battle_page(room, b), None);
            }
            match me {
                Some(id) if room.active() == Some(id) => turn(t, id),
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

fn lobby(t: T, me: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
    let mut items = vec![
        section(
            "Invitez vos amis",
            Stack::column()
                .gap(Gap::Sm)
                .children([
                    Stack::row()
                        .justify(StackJustify::Between)
                        .align_center()
                        .children([
                            el(El::Strong)
                                .st([St::Text3xl, St::TrackingWidest])
                                .text(t.code),
                            CopyButton::new(t.code).build(),
                        ])
                        .build(),
                    Text::caption(
                        "Partagez l'adresse de cette page, ou ce code à saisir sur l'accueil.",
                    )
                    .muted()
                    .build(),
                ])
                .build(),
        ),
        Text::body(
            "Choisissez votre royaume. Les royaumes sans seigneur seront joués par l'ordinateur.",
        )
        .muted()
        .build(),
    ];

    for (i, id) in KINGDOMS.into_iter().enumerate() {
        let k = room.game.kingdom(id);
        let (badge, action) = match room.seat(id).owner {
            Some(o) if o == t.token => (
                Badge::success("Vous").build(),
                Button::secondary("Quitter")
                    .size(ButtonSize::Sm)
                    .on_click(t.act(room::leave())),
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
                    .on_click(by(room::join(), t.token, t.code, &[i as u8])),
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
                t.act(room::rename()),
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

fn turn(t: T, id: Kingdoms) -> View {
    let room = t.room;
    let k = room.game.kingdom(id);
    let seat = room.seat(id);
    let mut items = vec![stepper(room.step)];
    if let Some(notice) = &seat.notice {
        items.push(Alert::info().message(notice.clone()).build());
    }
    let (body, action) = match room.step {
        Step::Weather => (weather_step(room, k), Some(next("Continuer", t))),
        Step::Trade => (trade_step(t, id), Some(next("Passer à l'intendance", t))),
        Step::Feed => (feed_step(t, k), None),
        Step::Report => (report_step(seat), Some(next("Continuer", t))),
        Step::Economy => (
            economy_step(t, k, seat),
            Some(next("Passer à la guerre", t)),
        ),
        Step::War => (war_step(t, id), Some(next("Fin du tour", t))),
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

fn weather_step(room: &Room, k: &Kingdom) -> ElementBuilder {
    Stack::column()
        .gap(Gap::Md)
        .children([
            Alert::info()
                .title(format!("An {}", room.game.year))
                .message(room.game.weather.sentence())
                .build(),
            resources(k),
        ])
        .build()
}

fn trade_step(t: T, id: Kingdoms) -> ElementBuilder {
    let room = t.room;
    let k = room.game.kingdom(id);
    let mut sellers = Select::new().name("seller");
    let mut offers = 0;
    let mut largest = 0;
    for other in room.game.alive_kingdoms().into_iter().filter(|&o| o != id) {
        let s = room.game.kingdom(other);
        if s.grain_to_sell < 1 || s.grain_price < 1 {
            continue;
        }
        offers += 1;
        largest = largest.max(s.grain_to_sell);
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
            t.act(room::buy_grain()),
            [
                labeled("Vendeur", sellers.build()),
                slider(
                    "buy_amount",
                    "Boisseaux (courtage 10 %)",
                    1,
                    largest.min(500),
                    100,
                    "boisseaux",
                ),
                Button::secondary("Acheter").full_width(true).build(),
            ],
        )
    };
    let stocks = k.grain_stocks.max(1);
    Stack::column()
        .gap(Gap::Md)
        .children([
            resources(k),
            section("Acheter du grain", buy),
            section(
                "Vendre du grain",
                form(
                    t.act(room::sell_grain()),
                    [
                        slider(
                            "sell_amount",
                            "Boisseaux à vendre",
                            1,
                            stocks,
                            (stocks / 10).max(1),
                            "boisseaux",
                        ),
                        slider(
                            "price",
                            "Prix du boisseau",
                            1,
                            MAX_GRAIN_PRICE,
                            5,
                            k.currency(),
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
                    t.act(room::sell_land()),
                    [
                        slider(
                            "arpents",
                            "Arpents (2 pièces l'arpent)",
                            1,
                            (k.surface / 2).max(1),
                            (k.surface / 100).max(1),
                            "arpents",
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

fn feed_step(t: T, k: &Kingdom) -> ElementBuilder {
    let stocks = k.grain_stocks.max(0);
    let needs = k.peasants_grain_needs();
    let army = k.soldiers_grain_needs();
    // Beyond 2× the people's needs immigration barely grows; beyond 1.5× the
    // army's needs efficiency is already maxed — so the sliders stop there.
    let peasants_max = (needs * 2).min(stocks).max(0);
    let soldiers_max = (army * 3 / 2).min(stocks).max(0);
    Stack::column()
        .gap(Gap::Md)
        .children([
            resources(k),
            section(
                "Nourrir le royaume",
                form(
                    t.act(room::feed()),
                    [
                        slider(
                            "peasants",
                            format!(
                                "Grain pour les {} habitants (besoin : {})",
                                fmt(k.population()),
                                fmt(needs)
                            ),
                            0,
                            peasants_max,
                            needs.min(peasants_max),
                            "boisseaux",
                        ),
                        slider(
                            "soldiers",
                            format!(
                                "Grain pour l'ost de {} hommes (besoin : {})",
                                fmt(k.soldiers),
                                fmt(army)
                            ),
                            0,
                            soldiers_max,
                            army.min(soldiers_max),
                            "boisseaux",
                        ),
                        Text::caption(
                            "Mal nourris, serfs et soldats meurent ou désertent ; bien nourris, les étrangers immigrent et l'ost combat mieux.",
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

fn economy_step(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
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
        let mut tbl = Table::new()
            .headers(["Poste", "Nombre", "Profits"])
            .striped(true);
        for (label, count, profit) in rows {
            tbl = tbl.row(TableRow::new().cells([label.to_string(), fmt(count), fmt(profit)]));
        }
        items.push(section(
            format!("Revenus d'état : {} {}", fmt(e.net()), k.currency()),
            scroll(tbl.build()),
        ));
    }

    items.push(section(
        "Taux d'imposition",
        form(
            t.act(room::set_taxes()),
            [
                slider(
                    "customs",
                    "Droits de douane",
                    0,
                    50,
                    k.immigration_taxes,
                    "%",
                ),
                slider("sales", "Taxe commerciale", 0, 20, k.commercial_taxes, "%"),
                slider("income", "Impôts directs", 0, 35, k.income_taxes, "%"),
                Button::secondary("Promulguer").full_width(true).build(),
            ],
        ),
    ));

    let kind = seat.invest_kind.unwrap_or(InvestmentType::Marketplaces);
    let max = kind.max_investment(k).max(0);
    let mut kinds = Select::new()
        .name("kind")
        .value(kind_number(kind).to_string());
    for (n, other) in (1..=6).filter_map(|n| InvestmentType::from_number(n).map(|k| (n, k))) {
        kinds = kinds.option(
            n.to_string(),
            format!(
                "{} — {} {} pièce (max {})",
                invest_fr(other),
                fmt(other.cost()),
                k.currency(),
                other.max_investment(k).max(0)
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
            t.act(room::invest()),
            [
                labeled("Type", kinds.on_change(t.act(room::pick_investment()))),
                slider(
                    "invest_amount",
                    format!(
                        "Quantité (max {max} à {} {} pièce)",
                        fmt(kind.cost()),
                        k.currency()
                    ),
                    0,
                    max,
                    1.min(max),
                    invest_fr(kind),
                ),
                Button::secondary("Investir")
                    .full_width(true)
                    .disabled(max < 1)
                    .build(),
            ],
        ),
    ));

    Stack::column().gap(Gap::Md).children(items).build()
}

/// Menu number of an investment type (1..=6), the inverse of `from_number`.
fn kind_number(kind: InvestmentType) -> i32 {
    (1..=6)
        .find(|&n| InvestmentType::from_number(n) == Some(kind))
        .unwrap_or(1)
}

fn war_step(t: T, id: Kingdoms) -> ElementBuilder {
    let room = t.room;
    let k = room.game.kingdom(id);
    let year = room.game.year;
    let mut tbl = Table::new()
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
        tbl = tbl.row(TableRow::new().cells([o.full_title(), fmt(o.surface), fmt(o.soldiers)]));
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
            scroll(tbl.build()),
            section(
                "Expédition",
                form(
                    t.act(room::attack()),
                    [
                        labeled("Cible", targets.build()),
                        slider(
                            "soldiers",
                            format!("Hommes d'armes (vous en avez {})", fmt(k.soldiers)),
                            1,
                            k.soldiers.max(1),
                            (k.soldiers / 2).max(1),
                            "hommes",
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

fn slider(
    name: &'static str,
    label: impl Into<Label>,
    min: i32,
    max: i32,
    value: i32,
    unit: impl Into<Label>,
) -> ElementBuilder {
    Slider::new()
        .name(name)
        .id(name)
        .label(label)
        .unit(unit)
        .min(min)
        .max(max.max(min))
        .value(value)
        .build()
}

fn primary(label: &'static str, spec: HandlerSpec) -> ElementBuilder {
    Button::primary(label)
        .size(ButtonSize::Lg)
        .full_width(true)
        .on_click(spec)
}

fn next(label: &'static str, t: T) -> ElementBuilder {
    primary(label, t.act(room::advance()))
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
