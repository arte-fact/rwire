//! Mobile-first views over the shared [`Rooms`]. Pure functions: `page` is
//! re-run for every connection whenever any table changes.

use std::borrow::Cow;

use empire_lib::demography::{army_losses_share, demography_outlook, Council, Outlook};
use empire_lib::economy::{economy_outlook, Taxes};
use empire_lib::events::RulerDeathCause;
use empire_lib::investments::InvestmentType;
use empire_lib::kingdom::RATION_SCALE;
use empire_lib::trade::{
    calculate_buy_cost, grain_value, max_land_sale, LAND_SELL_PRICE, MAX_GRAIN_PRICE,
};
use empire_lib::{Criterion, Fate, Kingdom, Kingdoms, PlayerTitle, Requirement, Weather, KINGDOMS};
use rwire::attr_tokens::{At, Av};
use rwire::builder::LiveSum;
use rwire::{el, icon_sized, El, ElementBuilder, Ev, HandlerSpec, Icon, St, Style};
use rwire_components::{
    Alert, Badge, Button, ButtonIntent, ButtonSize, CopyButton, Drawer, DrawerPosition, Gap, Grid,
    GridColumns, Input, Link, Progress, ProgressIntent, Slider, Stack, StackJustify, Stat, Stepper,
    Text, TextVariant,
};

use crate::room::{
    self, buildings_fr, by, goods_fr, people_fr, Battle, Deal, Draft, Elsewhere, Entry, Field,
    News, Phase, Room, Rooms, Seat, Side, Spot, Stage, Step,
};

type Label = Cow<'static, str>;

/// Bottom tabs.
const TABS: [&str; 3] = ["Partie", "Royaumes", "Journal"];

/// An open bottom sheet, pinned to the step/year it was opened for and to
/// the seat's sheet generation (the form concluding bumps it, which closes
/// the sheet).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet {
    pub action: u8,
    pub step: u8,
    pub year: u16,
    pub gen: u16,
}

/// Step tag of a sheet that is not tied to a step of the year.
const NO_STEP: u8 = 0xFF;

/// Everything that lives in the bottom sheet: the two seat forms and, on the
/// Intendance roll, every decision of the year.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Rename,
    Title,
    /// Buy grain from a seller.
    Buy(Kingdoms),
    /// List grain on the market.
    Sell,
    /// Sell land to the barbarians.
    Land,
    /// One of the six purchases.
    Invest(InvestmentType),
    /// One of the council's sliders.
    Council(Field),
}

impl Action {
    /// The wire code of the action (a byte of the sheet's param bytes).
    fn code(self) -> u8 {
        match self {
            Action::Rename => 1,
            Action::Title => 2,
            Action::Buy(o) => 0x10 + o.index() as u8,
            Action::Sell => 0x20,
            Action::Land => 0x21,
            Action::Invest(kind) => 0x30 + kind as u8 + 1,
            Action::Council(field) => 0x40 + field as u8,
        }
    }

    fn from_u8(n: u8) -> Option<Action> {
        match n {
            1 => Some(Action::Rename),
            2 => Some(Action::Title),
            0x10..=0x15 => Kingdoms::from_number(i32::from(n - 0x10) + 1).map(Action::Buy),
            0x20 => Some(Action::Sell),
            0x21 => Some(Action::Land),
            0x31..=0x36 => InvestmentType::from_number(i32::from(n - 0x30)).map(Action::Invest),
            0x40..=0x44 => Field::from_u8(n - 0x40).map(Action::Council),
            _ => None,
        }
    }

    fn title(self) -> String {
        match self {
            Action::Rename => "Votre nom".to_string(),
            Action::Title => "Votre titre".to_string(),
            Action::Buy(o) => format!("Acheter à la {}", o.name()),
            Action::Sell => "Vendre du grain".to_string(),
            Action::Land => "Vendre des terres".to_string(),
            Action::Invest(kind) => match kind {
                InvestmentType::Marketplaces => "Champ de foire",
                InvestmentType::GrainMills => "Moulin à grain",
                InvestmentType::Foundries => "Fonderie",
                InvestmentType::Shipyards => "Chantier naval",
                InvestmentType::Soldiers => "Recruter",
                InvestmentType::Palaces => "Palais",
            }
            .to_string(),
            Action::Council(field) => match field {
                Field::Peasants => "Ration du peuple",
                Field::Soldiers => "Ration de l'ost",
                Field::Customs => "Droits de douane",
                Field::Sales => "Gabelle",
                Field::Income => "Taille",
            }
            .to_string(),
        }
    }

    /// The step this action belongs to (`None` = any time while seated).
    fn step(self) -> Option<Step> {
        match self {
            Action::Rename | Action::Title => None,
            _ => Some(Step::Intendance),
        }
    }

    /// Only makes sense before the game starts.
    fn lobby_only(self) -> bool {
        self == Action::Rename
    }
}

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

pub fn page(
    rooms: &Rooms,
    token: u64,
    tab: u8,
    code: Option<&str>,
    sheet: Option<Sheet>,
) -> ElementBuilder {
    match code.and_then(|c| rooms.get(c).map(|r| (c, r))) {
        Some((code, room)) => room_page(T { room, code, token }, tab, sheet),
        None => home_page(rooms, token, code.is_some()),
    }
}

/// App shell: the root is exactly one dynamic viewport tall and never scrolls;
/// `main` is the scroll container and the bar sits in normal flow below it. No
/// `position: fixed`, so a collapsing mobile address bar can't hide or jolt it.
/// `panel` sits between the header and `main`, out of the scroll.
fn shell(
    header: ElementBuilder,
    content: ElementBuilder,
    bar: ElementBuilder,
    overlay: Option<ElementBuilder>,
) -> ElementBuilder {
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
            overlay.unwrap_or_else(|| el(El::Div)),
        ])
}

// ---------------------------------------------------------------------------
// Home: create a table or join one
// ---------------------------------------------------------------------------

fn home_page(rooms: &Rooms, token: u64, unknown: bool) -> ElementBuilder {
    let mut join = vec![
        Input::text()
            .name("code")
            .id("code")
            .placeholder("CODE")
            .autocomplete("off")
            .spellcheck(false)
            .autocapitalize(Av::Characters)
            .maxlength(5)
            .required(true)
            .build()
            .st([
                St::TextCenter,
                St::TextUppercase,
                St::TrackingWidest,
                St::TextLg,
            ]),
        Button::secondary("Rejoindre la table")
            .full_width(true)
            .build(),
    ];
    if unknown {
        join.push(
            Text::caption("Table introuvable : cette table n'existe plus ou le code est erroné.")
                .build()
                .st([St::TextError, St::TextCenter]),
        );
    }
    let mut items = vec![
        el(El::Div).st([St::TextCenter]).append([
            Text::new()
                .variant(TextVariant::Heading1)
                .content("EMPIRE")
                .build()
                .st([St::TrackingWidest]),
            Text::body(
                "Six royaumes, un seul empereur. Les sièges vides sont tenus par l'ordinateur.",
            )
            .muted()
            .build(),
        ]),
        primary("Créer une table", by(room::create_room(), token, "", &[])),
        Text::caption("— ou —").muted().build().st([St::TextCenter]),
        form(by(room::enter_code(), token, "", &[]), join),
    ];
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
    // No header, no bar: the two gestures sit together, centred in the screen.
    el(El::Div)
        .st([
            St::HDvh,
            St::BgApp,
            St::TextDefault,
            St::OverflowYAuto,
            St::DisplayFlex,
            St::FlexCol,
        ])
        .append([el(El::Div)
            .st([
                St::MaxWMd,
                St::MxAuto,
                St::MyAuto,
                St::WFull,
                St::PMd,
                St::DisplayFlex,
                St::FlexCol,
                St::GapLg,
            ])
            .append(items)])
}

fn room_page(t: T, tab: u8, sheet: Option<Sheet>) -> ElementBuilder {
    let me = t.room.seat_of(t.token);
    let open = sheet.and_then(|sh| open_action(t, me, sh));
    let war = war_target(t, me);
    let View { body, action } = match tab {
        1 => View::new(kingdoms_tab(t), None),
        2 => View::new(journal_tab(t), None),
        _ => partie(t, me, open.is_some() || war.is_some()),
    };
    let overlay = match (open, war) {
        (Some((id, act)), _) => Some(drawer(
            t,
            id,
            act.title(),
            crate::close_sheet(),
            sheet_form(t, id, act),
        )),
        (None, Some((id, target))) => Some(drawer(
            t,
            id,
            campaign::party_name(target).to_string(),
            by(room::pick_target(), t.token, t.code, &[0xFF]),
            war_sheet(t, id, target),
        )),
        (None, None) => None,
    };
    let tabs = (t.room.stage != Stage::Lobby).then_some(tab);
    shell(header(t, me), body, bottom_bar(action, tabs), overlay)
}

/// The bottom sheet: the seat's notice for it, if any, over `form`.
fn drawer(
    t: T,
    id: Kingdoms,
    title: String,
    on_close: HandlerSpec,
    form: ElementBuilder,
) -> ElementBuilder {
    let mut body = Vec::new();
    let seat = t.room.seat(id);
    if let (Some(notice), Spot::Sheet) = (&seat.notice, seat.notice_spot) {
        body.push(Alert::info().message(notice.clone()).build());
    }
    body.push(form);
    Drawer::new()
        .position(DrawerPosition::Bottom)
        .open(true)
        .title(title)
        .on_close(on_close)
        .content(Stack::column().gap(Gap::Md).children(body).build())
        .build()
}

/// The sheet's action if it is still valid for this viewer at this moment.
fn open_action(t: T, me: Option<Kingdoms>, sh: Sheet) -> Option<(Kingdoms, Action)> {
    let id = me?;
    let act = Action::from_u8(sh.action)?;
    let room = t.room;
    if sh.gen != room.seat(id).sheet_gen {
        return None;
    }
    let valid = match act.step() {
        None => sh.step == NO_STEP && (room.stage == Stage::Lobby || !act.lobby_only()),
        Some(step) => {
            room.may_act(id, step)
                && sh.step == step.index() as u8
                && sh.year == room.game.year as u16
        }
    };
    valid.then_some((id, act))
}

/// Handler that opens the sheet for `act`, pinned to the current step, year
/// and sheet generation of the viewer's seat.
fn sheet_spec(t: T, act: Action) -> HandlerSpec {
    let step = act.step().map(|s| s.index() as u8).unwrap_or(NO_STEP);
    let year = (t.room.game.year as u16).to_le_bytes();
    let gen = t
        .room
        .seat_of(t.token)
        .map_or(0, |id| t.room.seat(id).sheet_gen)
        .to_le_bytes();
    crate::open_sheet().with_param_bytes(vec![act.code(), step, year[0], year[1], gen[0], gen[1]])
}

/// The form shown inside the sheet.
fn sheet_form(t: T, id: Kingdoms, act: Action) -> ElementBuilder {
    let k = t.room.game.kingdom(id);
    let seat = t.room.seat(id);
    match act {
        Action::Rename => rename_form(t, k),
        Action::Title => title_sheet(t, k),
        Action::Buy(o) => buy_sheet(t, k, seat, o),
        Action::Sell => sell_sheet(t, k, seat),
        Action::Land => land_sheet(t, k, seat),
        Action::Invest(kind) => purchase_sheet(t, id, kind),
        Action::Council(field) => council_sheet(t, id, field),
    }
}

fn header(t: T, me: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
    let (title, sub) = match me {
        Some(id) if room.stage != Stage::Lobby => {
            let k = room.game.kingdom(id);
            (
                title_button(t, k, format!("{} · {}", k.titled_name(), k.name())),
                if k.is_dead {
                    Text::caption(capitalize(&fate_fr(k))).muted().build()
                } else {
                    resources_line(k)
                },
            )
        }
        Some(id) => (
            title_button(t, room.game.kingdom(id), room.game.kingdom(id).full_title()),
            Text::caption(format!(
                "En attente · {} seigneur(s), {} ordinateur(s)",
                room.humans().count(),
                6 - room.humans().count()
            ))
            .muted()
            .build(),
        ),
        None => (
            el(El::Strong)
                .st([St::DisplayBlock, St::Truncate])
                .text(&format!("Table {}", t.code)),
            Text::caption("Spectateur").muted().build(),
        ),
    };
    header_bar(
        title,
        sub,
        (room.stage != Stage::Lobby)
            .then(|| Badge::primary(format!("An {}", room.game.year)).build()),
        Some(
            Button::icon_only(Icon::Home, "Accueil")
                .intent(ButtonIntent::Ghost)
                .size(ButtonSize::Sm)
                .on_click(crate::go_home()),
        ),
    )
}

/// The header's name line: the rank mark and the titled name, as a flat
/// button that opens the title sheet.
fn title_button(t: T, k: &Kingdom, name: String) -> ElementBuilder {
    el(El::Button)
        .st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapXs,
            St::MaxWFull,
            St::P0,
            St::BgTransparent,
            St::BorderNone,
            St::TextDefault,
            St::FontInheritAll,
            St::FontSemibold,
            St::CursorPointer,
            St::TextLeft,
        ])
        .hover([St::TextAccent])
        .at(At::Type, Av::Button)
        .at_str(At::AriaLabel, "Votre titre et la marche vers le prochain")
        .append([
            rank_mark(k, 20, St::TextDefault),
            el(El::Span).st([St::Truncate]).text(&name),
        ])
        .on(Ev::Click, sheet_spec(t, Action::Title))
}

fn rank_icon(title: PlayerTitle) -> Icon {
    match title {
        PlayerTitle::Duke => Icon::Circlet,
        PlayerTitle::Prince => Icon::Coronet,
        PlayerTitle::King => Icon::Crown,
        PlayerTitle::Emperor => Icon::ImperialCrown,
    }
}

/// The heraldic crown of a kingdom's current rank. The two lower ranks take
/// `low` (the surrounding text's colour); a Roy or an Empereur lights up in
/// the accent.
fn rank_mark(k: &Kingdom, size: u32, low: St) -> ElementBuilder {
    let title = k.title();
    // Decorative: the rank is always spelled out beside it ("Roy Hugues"), so
    // naming the mark would read the title twice.
    icon_sized(rank_icon(title), size)
        .st([if title >= PlayerTitle::King {
            St::TextAccent
        } else {
            low
        }])
        .at(At::AriaHidden, Av::True)
}

/// "annexée par la France" / "en déshérence, Sir Arthur assassiné" — how a
/// fallen realm fell, for the lists.
fn fate_fr(k: &Kingdom) -> String {
    match k.fate {
        Some(Fate::Annexed(by)) => format!("annexée par la {}", by.name()),
        Some(Fate::RulerDied(cause)) => format!(
            "en déshérence, {} {}",
            k.player_name,
            match cause {
                RulerDeathCause::Assassination | RulerDeathCause::StarvationAssassination =>
                    "assassiné",
                RulerDeathCause::HuntingAccident => "mort à la chasse",
                RulerDeathCause::FoodPoisoning => "empoisonné",
                RulerDeathCause::NaturalCauses | RulerDeathCause::None => "mort",
            }
        ),
        None => "tombée".to_string(),
    }
}

/// "⚜ France · Roy Hugues" for the kingdom lists; `suffix` follows the name
/// in the muted run (" (ordinateur)").
fn ruler(k: &Kingdom, suffix: &str) -> ElementBuilder {
    el(El::Span).st([St::MinW0]).append([
        rank_mark(k, 15, St::TextMuted).st([St::MrXs]),
        el(El::Strong).text(k.name()),
        el(El::Span)
            .st([St::TextMuted])
            .text(&format!(" · {}{suffix}", k.titled_name())),
    ])
}

/// "1 000 francs · 33 923 boisseaux · 20 hommes d'armes" on one line, numbers bold.
fn resources_line(k: &Kingdom) -> ElementBuilder {
    let figure = |n: i32, unit: String| {
        el(El::Span).append([
            el(El::Strong).st([St::TextDefault]).text(&fmt(n)),
            el(El::Span).text(&format!(" {unit}")),
        ])
    };
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::GapSm,
            St::TextXs,
            St::TextMuted,
            St::WhitespaceNowrap,
            St::TabularNums,
            St::OverflowHidden,
        ])
        .append([
            figure(k.treasury, coin_word(k.treasury, k.currency()).to_string()),
            figure(k.grain_stocks, "boisseaux".to_string()),
            figure(k.soldiers, "hommes d'armes".to_string()),
        ])
}

fn header_bar(
    title: ElementBuilder,
    sub: ElementBuilder,
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
                el(El::Div).st([St::MinW0]).append([title, sub]),
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
            el(El::Div)
                .st([St::DisplayFlex])
                .at(At::Role, Av::RoleTablist)
                .append(TABS.iter().enumerate().map(|(i, label)| {
                    let on = i as u8 == tab;
                    el(El::Button)
                        .st([
                            St::Flex1,
                            St::BgTransparent,
                            St::BorderNone,
                            St::TextSm,
                            St::TextCenter,
                            St::CursorPointer,
                            St::PySm,
                            if on { St::TextDefault } else { St::TextMuted },
                            if on { St::FontSemibold } else { St::FontNormal },
                            if on {
                                St::BorderB2Accent
                            } else {
                                St::BorderB2Transparent
                            },
                        ])
                        .at(At::Role, Av::RoleTab)
                        .at(At::AriaSelected, if on { Av::True } else { Av::False })
                        .text(label)
                        .on(Ev::Click, crate::set_tab().with_param_bytes(vec![i as u8]))
                })),
        );
    }
    if rows.is_empty() {
        return el(El::Div);
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

fn journal_tab(t: T) -> ElementBuilder {
    let room = t.room;
    if room.log.is_empty() {
        return Text::body("Le journal est encore vierge.").muted().build();
    }
    let me = room.seat_of(t.token);
    // Grouped by year, most recent year first; within a year the chronicle
    // reads in order.
    let mut years: Vec<(i32, Vec<&Entry>)> = Vec::new();
    for e in &room.log {
        match years.last_mut() {
            Some((y, v)) if *y == e.year => v.push(e),
            _ => years.push((e.year, vec![e])),
        }
    }
    Stack::column()
        .gap(Gap::Md)
        .children(years.into_iter().rev().map(|(year, entries)| {
            section(
                format!("An {year}"),
                el(El::Div).append(entries.into_iter().map(|e| journal_line(e, me))),
            )
        }))
        .build()
}

/// One journal line; an amber dot marks news about the viewer's own kingdom.
fn journal_line(e: &Entry, me: Option<Kingdoms>) -> ElementBuilder {
    let mine = me.is_some_and(|m| e.about.contains(&m));
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::ItemsBaseline,
            St::GapSm,
            St::PyXs,
            St::TextSm,
        ])
        .append([
            el(El::Span)
                .st([St::W05rem, St::H05rem, St::RoundedFull, St::FlexShrink0])
                .st(if mine { [St::BgWarning] } else { [St::BgMuted] }),
            el(El::Span).text(&e.text),
        ])
}

/// The last few journal entries, in order, for screens where the viewer is waiting.
fn latest_news(t: T, n: usize) -> ElementBuilder {
    let me = t.room.seat_of(t.token);
    let log = &t.room.log;
    let lines: Vec<ElementBuilder> = log[log.len().saturating_sub(n)..]
        .iter()
        .map(|e| journal_line(e, me))
        .collect();
    if lines.is_empty() {
        return el(El::Div);
    }
    section("Derniers événements", el(El::Div).append(lines))
}

fn kingdoms_tab(t: T) -> ElementBuilder {
    let room = t.room;
    let me = room.seat_of(t.token);
    let most = KINGDOMS
        .into_iter()
        .map(|id| room.game.kingdom(id).surface)
        .max()
        .unwrap_or(1)
        .max(1);
    let mut cards: Vec<ElementBuilder> = vec![Grid::new()
        .columns(GridColumns::Fixed2)
        .gap(Gap::Sm)
        .children([
            Stat::new(fmt(room
                .game
                .kingdoms
                .iter()
                .map(|k| k.surface)
                .sum::<i32>()))
            .label("Terres tenues")
            .build(),
            Stat::new(room.game.alive_kingdoms().len().to_string())
                .label("Royaumes en lice")
                .build(),
        ])
        .build()];
    let mut alive: Vec<Kingdoms> = room.game.alive_kingdoms();
    alive.sort_by_key(|&id| std::cmp::Reverse(room.game.kingdom(id).surface));
    let mut rows: Vec<ElementBuilder> = alive
        .into_iter()
        .map(|id| kingdom_row(room, id, me == Some(id), most))
        .collect();
    rows.extend(
        KINGDOMS
            .into_iter()
            .map(|id| room.game.kingdom(id))
            .filter(|k| k.is_dead)
            .map(|k| {
                Text::caption(format!("{} · {}", k.name(), fate_fr(k)))
                    .muted()
                    .build()
                    .st([St::PySm, St::PlSm, St::BorderB])
            }),
    );
    cards.push(section("Royaumes", el(El::Div).append(rows)));
    cards.push(market_section(t, me));
    Stack::column().gap(Gap::Md).children(cards).build()
}

/// One kingdom: name and ruler, its land against the largest realm, and the
/// four headcounts that make up its strength. The viewer's own line carries
/// the accent left border.
fn kingdom_row(room: &Room, id: Kingdoms, mine: bool, most: i32) -> ElementBuilder {
    let k = room.game.kingdom(id);
    let figure = |n: i32, label: &'static str| {
        el(El::Span).st([St::WhitespaceNowrap]).append([
            el(El::Strong).st([St::TabularNums]).text(&fmt(n)),
            el(El::Span).st([St::TextMuted]).text(&format!(" {label}")),
        ])
    };
    let mut title = vec![ruler(
        k,
        if room.is_computer(id) {
            " · ordinateur"
        } else {
            ""
        },
    )];
    if mine {
        title.push(Badge::new().text("vous").build().st([St::MlSm]));
    }
    el(El::Div)
        .st([St::PySm, St::PlSm, St::BorderB])
        .st(if mine {
            [St::BorderL3Accent]
        } else {
            [St::BorderL3Transparent]
        })
        .append([
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::JustifyBetween,
                    St::ItemsBaseline,
                    St::GapSm,
                ])
                .append([
                    el(El::Span).st([St::MinW0]).append(title),
                    figure(k.surface, "arpents"),
                ]),
            Progress::new()
                .value(k.surface.max(0) as u32)
                .max(most as u32)
                .thin(true)
                .build()
                .st([St::MtXs]),
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::FlexWrap,
                    St::GapMd,
                    St::TextSm,
                    St::MtXs,
                ])
                .append([
                    figure(k.nobles, "nobles"),
                    figure(k.soldiers, "soldats"),
                    figure(k.merchants, "marchands"),
                    figure(k.peasants, "serfs"),
                ]),
        ])
}

// ---------------------------------------------------------------------------
// Partie tab
// ---------------------------------------------------------------------------

/// What a tab shows: the scrolling body, the bottom bar's primary action and
/// an optional panel pinned under the header.
struct View {
    body: ElementBuilder,
    action: Option<ElementBuilder>,
}

impl View {
    fn new(body: ElementBuilder, action: Option<ElementBuilder>) -> View {
        View { body, action }
    }
}

fn partie(t: T, me: Option<Kingdoms>, sheet_open: bool) -> View {
    let room = t.room;
    match room.stage {
        Stage::Lobby => View::new(
            lobby(t, me),
            me.map(|_| primary("Commencer la partie", t.act(room::start()))),
        ),
        Stage::Over => View::new(
            over(room),
            me.map(|_| primary("Nouvelle partie", t.act(room::new_game()))),
        ),
        Stage::Playing => {
            if room.phase == Phase::Campaign {
                return campaign::campaign_page(t, me);
            }
            match me {
                Some(id) if room.playing(id) => turn(t, id, sheet_open),
                // A fallen seigneur keeps the tale of their fall, then
                // follows the table.
                Some(id) if room.game.kingdom(id).is_dead => View::new(
                    Stack::column()
                        .gap(Gap::Md)
                        .children([
                            chronicle(room, id, "Votre chronique", 0),
                            Text::caption("Vous suivez désormais la partie en spectateur.")
                                .muted()
                                .build(),
                            waiting(t),
                        ])
                        .build(),
                    None,
                ),
                // Seated players simply wait; only unseated viewers (a lost
                // session, a spectator) are offered the seat-recovery list.
                Some(_) => View::new(waiting(t), None),
                None => View::new(
                    Stack::column()
                        .gap(Gap::Md)
                        .children([reclaim_section(t), waiting(t)])
                        .build(),
                    None,
                ),
            }
        }
    }
}

/// Offered to unseated viewers of a running table: take back a human seat
/// (the recovery path after a lost session on another device or app).
fn reclaim_section(t: T) -> ElementBuilder {
    let seats: Vec<ElementBuilder> = KINGDOMS
        .into_iter()
        .filter(|&id| t.room.seat(id).owner.is_some())
        .map(|id| {
            let k = t.room.game.kingdom(id);
            Button::secondary(format!("{} ({}) — c'est moi", id.name(), k.player_name))
                .full_width(true)
                .on_click(by(room::reclaim(), t.token, t.code, &[id.index() as u8]))
        })
        .collect();
    if seats.is_empty() {
        return el(El::Div);
    }
    section(
        "Reprendre un siège",
        Stack::column().gap(Gap::Sm).children(seats).build(),
    )
}

fn lobby(t: T, me: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
    // The code is the screen's one figure: centred, large, the copy beside it.
    let invite = el(El::Div).st([St::TextCenter, St::PyMd]).append([
        eyebrow("Code d'invitation"),
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::JustifyCenter,
                St::ItemsCenter,
                St::GapSm,
            ])
            .append([
                el(El::Strong)
                    .st([St::Text4xl, St::TrackingWidest, St::TabularNums])
                    .text(t.code),
                CopyButton::new(t.code).build(),
            ]),
    ]);

    let row_tokens = [
        St::DisplayFlex,
        St::JustifyBetween,
        St::ItemsCenter,
        St::GapSm,
        St::WFull,
        St::PySm,
        St::PlSm,
        St::BorderB,
        St::BorderL3Transparent,
        St::TextSm,
        St::TextLeft,
    ];
    let seats = KINGDOMS.into_iter().enumerate().map(|(i, id)| {
        let k = room.game.kingdom(id);
        let name = |ruler: &str| {
            el(El::Span).st([St::MinW0]).append([
                rank_mark(k, 15, St::TextMuted).st([St::MrXs]),
                el(El::Strong).text(id.name()),
                el(El::Span)
                    .st([St::TextMuted])
                    .text(&format!(" · {ruler}")),
            ])
        };
        match room.seat(id).owner {
            Some(o) if o == t.token => {
                el(El::Div).st(row_tokens).st([St::BorderL3Accent]).append([
                    name(&k.titled_name()),
                    Stack::row()
                        .gap(Gap::Sm)
                        .align_center()
                        .children([
                            Badge::success("Vous").build(),
                            Button::ghost("Nom")
                                .size(ButtonSize::Sm)
                                .on_click(sheet_spec(t, Action::Rename)),
                            Button::ghost("Quitter")
                                .size(ButtonSize::Sm)
                                .on_click(t.act(room::leave())),
                        ])
                        .build(),
                ])
            }
            // A seat held by another session: an unseated viewer may take it
            // back (their own seat after a lost session — trust rules).
            Some(_) => el(El::Div).st(row_tokens).append([
                name(&k.titled_name()),
                if me.is_none() {
                    Stack::row()
                        .gap(Gap::Sm)
                        .align_center()
                        .children([
                            Badge::warning("Pris").build(),
                            Button::ghost("C'est moi").size(ButtonSize::Sm).on_click(by(
                                room::reclaim(),
                                t.token,
                                t.code,
                                &[i as u8],
                            )),
                        ])
                        .build()
                } else {
                    Badge::warning("Pris").build()
                },
            ]),
            // The whole line is the button: tap a seat to take it.
            None => el(El::Button)
                .st(row_tokens)
                .st([
                    St::BgTransparent,
                    St::BorderNone,
                    St::BorderB,
                    St::BorderL3Transparent,
                    St::Pr0,
                    St::TextDefault,
                    St::FontInheritAll,
                    St::CursorPointer,
                ])
                .hover([St::BgSubtle])
                .at(At::Type, Av::Button)
                .append([
                    name(&format!(
                        "{} {}",
                        id.title_name(PlayerTitle::Duke),
                        id.default_king_name()
                    )),
                    Badge::default_badge("Ordinateur").build(),
                ])
                .on(Ev::Click, by(room::join(), t.token, t.code, &[i as u8])),
        }
    });
    let hint = if me.is_some() {
        "Les autres joueurs rejoignent avec le lien ; la partie peut démarrer à tout moment."
    } else {
        "Touchez un siège pour le prendre ; les sièges vides sont tenus par l'ordinateur."
    };

    Stack::column()
        .gap(Gap::Md)
        .children([
            invite,
            section(
                "Royaumes",
                el(El::Div).append([
                    el(El::Div).append(seats),
                    Text::caption(hint).muted().build().st([St::MtSm]),
                ]),
            ),
        ])
        .build()
}

/// One stroke per kingdom on the land curve: `var(--…)` colors so the chart
/// follows the theme.
const CURVE_COLORS: [&str; 6] = [
    "var(--U9)",
    "var(--O9)",
    "var(--P9)",
    "var(--M9)",
    "var(--n9)",
    "var(--N9)",
];

fn over(room: &Room) -> ElementBuilder {
    let mut standing: Vec<&Kingdom> = room.game.kingdoms.iter().filter(|k| !k.is_dead).collect();
    standing.sort_by_key(|k| std::cmp::Reverse((k.surface, k.total_population())));
    let winner = standing
        .first()
        .map(|k| format!("{} règne sur {} arpents.", k.full_title(), fmt(k.surface)))
        .unwrap_or_else(|| "Tous les royaumes sont tombés.".to_string());
    let most = standing.first().map_or(1, |k| k.surface.max(1));
    let peak = |id: Kingdoms| {
        room.history
            .iter()
            .map(|h| h[id.index()])
            .max()
            .unwrap_or(0)
    };
    let row = |rank: &str, k: &Kingdom, note: &str, figures: String, bar: Option<u32>| {
        let mut children = vec![el(El::Div)
            .st([St::DisplayFlex, St::ItemsBaseline, St::GapSm, St::TextSm])
            .append([
                el(El::Span)
                    .st([St::TextMuted, St::TabularNums, St::W1rem])
                    .text(rank),
                ruler(k, note).st([St::Flex1]),
                el(El::Span)
                    .st([St::TextMuted, St::TabularNums, St::WhitespaceNowrap])
                    .text(&figures),
            ])];
        if let Some(surface) = bar {
            children.push(
                Progress::new()
                    .value(surface)
                    .max(most as u32)
                    .thin(true)
                    .build()
                    .st([St::MtXs]),
            );
        }
        el(El::Div).st([St::PySm, St::BorderB]).append(children)
    };
    let mut rows: Vec<ElementBuilder> = standing
        .iter()
        .enumerate()
        .map(|(i, k)| {
            row(
                &(i + 1).to_string(),
                k,
                "",
                format!(
                    "{} arpents · {} sujets",
                    fmt(k.surface),
                    fmt(k.total_population())
                ),
                Some(k.surface.max(0) as u32),
            )
        })
        .collect();
    rows.extend(
        KINGDOMS
            .into_iter()
            .map(|id| (id, room.game.kingdom(id)))
            .filter(|(_, k)| k.is_dead)
            .map(|(id, k)| {
                row(
                    "—",
                    k,
                    &format!(" · {}", fate_fr(k)),
                    format!("{} au plus haut", fmt(peak(id))),
                    None,
                )
                .st([St::Opacity75])
            }),
    );
    let mut children = vec![
        Alert::success()
            .title("Fin de la partie")
            .message(winner)
            .build(),
        section("Classement", el(El::Div).append(rows)),
    ];
    if room.history.len() > 1 {
        children.push(section("Terres au fil des ans", land_curve(room)));
    }
    Stack::column().gap(Gap::Md).children(children).build()
}

/// Every kingdom's surface year by year, one stroke each, with a legend.
fn land_curve(room: &Room) -> ElementBuilder {
    const W: f64 = 320.0;
    const H: f64 = 100.0;
    const PAD: f64 = 4.0;
    let years = room.history.len();
    let top = room
        .history
        .iter()
        .flat_map(|h| h.iter().copied())
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let x = |i: usize| (i as f64 / (years - 1).max(1) as f64) * W;
    let y = |s: i32| H - PAD - (s.max(0) as f64 / top) * (H - 2.0 * PAD);
    let grid = format!(
        "M0 {a}H{W}M0 {b}H{W}M0 {c}H{W}",
        a = y(0),
        b = y((top / 2.0) as i32),
        c = y(top as i32)
    );
    let mut svg = el(El::Svg)
        .at_str(At::ViewBox, &format!("0 0 {W} {H}"))
        .at_str(At::Width, "100%")
        .st([St::DisplayBlock])
        .append([el(El::Path)
            .at_str(At::D, &grid)
            .at(At::Fill, Av::None)
            .at_str(At::Stroke, "var(--h)")
            .at_str(At::StrokeWidth, "1")]);
    let mut legend = Vec::new();
    for id in KINGDOMS {
        let i = id.index();
        let d: String = room
            .history
            .iter()
            .enumerate()
            .map(|(n, h)| {
                format!(
                    "{}{:.1} {:.1}",
                    if n == 0 { "M" } else { "L" },
                    x(n),
                    y(h[i])
                )
            })
            .collect();
        svg = svg.append([el(El::Path)
            .at_str(At::D, &d)
            .at(At::Fill, Av::None)
            .at_str(At::Stroke, CURVE_COLORS[i])
            .at_str(At::StrokeWidth, "2")
            .at(At::StrokeLinejoin, Av::Round)
            .at(At::StrokeLinecap, Av::Round)]);
        legend.push(
            el(El::Span)
                .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
                .append([
                    el(El::Span)
                        .st([St::W05rem, St::H05rem, St::RoundedFull, St::FlexShrink0])
                        .style(Style::new().background(CURVE_COLORS[i])),
                    el(El::Span).text(id.name()),
                ]),
        );
    }
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapSm])
        .append([
            svg,
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::JustifyBetween,
                    St::TextXs,
                    St::TextMuted,
                    St::TabularNums,
                ])
                .append([
                    el(El::Span).text("An 1"),
                    el(El::Span).text(&format!("An {}", room.game.year)),
                ]),
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::FlexWrap,
                    St::GapSm,
                    St::TextXs,
                    St::TextMuted,
                ])
                .append(legend),
        ])
}

fn waiting(t: T) -> ElementBuilder {
    match t.room.phase {
        Phase::Intendance => waiting_intendance(t),
        Phase::Exterieur | Phase::Campaign => waiting_exterieur(t),
    }
}

/// One line of the kingdoms list on the waiting screens: name, seigneur, badge.
fn waiting_row(room: &Room, id: Kingdoms, badge: Badge) -> ElementBuilder {
    let k = room.game.kingdom(id);
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsCenter,
            St::GapSm,
            St::PyXs,
            St::TextSm,
        ])
        .append([
            ruler(
                k,
                if room.is_computer(id) {
                    " (ordinateur)"
                } else {
                    ""
                },
            ),
            badge.build(),
        ])
}

/// Everyone runs their kingdom at once; the viewer has finished (or only
/// watches) and sees where the others stand.
fn waiting_intendance(t: T) -> ElementBuilder {
    let room = t.room;
    let me = room.seat_of(t.token);
    let alive = room.game.alive_kingdoms();
    let rows = alive.iter().map(|&id| {
        let seat = room.seat(id);
        let badge = if room.is_computer(id) || seat.ready {
            Badge::primary("Prêt")
        } else {
            Badge::default_badge(format!(
                "{} · {}/{}",
                seat.step.label(),
                seat.step.index() + 1,
                Step::LABELS.len()
            ))
        };
        waiting_row(room, id, badge)
    });
    let ready = me.is_some_and(|id| room.seat(id).ready && !room.game.kingdom(id).is_dead);
    let (title, caption) = if ready {
        (
            "Vous êtes prêt",
            "En attente des autres seigneurs ; la guerre s'ouvrira quand tous auront tenu conseil.",
        )
    } else {
        (
            "Intendance",
            "Chaque seigneur gouverne son royaume ; la guerre s'ouvrira quand tous seront prêts.",
        )
    };
    let mut head = vec![
        Text::new()
            .variant(TextVariant::Heading3)
            .content(title)
            .build(),
        Text::caption(caption).muted().build(),
    ];
    if ready {
        head.push(stepper(Step::War));
    }
    Stack::column()
        .gap(Gap::Md)
        .children([
            Stack::column().gap(Gap::Sm).children(head).build(),
            section("Royaumes", el(El::Div).append(rows)),
            latest_news(t, 3),
        ])
        .build()
}

/// Everyone gives their war orders at once; the viewer has given theirs (or
/// only watches) and sees who is still at it. Then the armies march.
fn waiting_exterieur(t: T) -> ElementBuilder {
    let room = t.room;
    let me = room.seat_of(t.token);
    let marching = room.phase == Phase::Campaign;
    let rows = room.game.alive_kingdoms().into_iter().map(|id| {
        let badge = if marching || room.is_computer(id) || room.seat(id).ready {
            Badge::primary("Prêt")
        } else {
            Badge::default_badge("Guerre")
        };
        waiting_row(room, id, badge)
    });
    let ready = me.is_some_and(|id| room.seat(id).ready && !room.game.kingdom(id).is_dead);
    let (title, caption) = if marching {
        (
            "Les armées marchent",
            "Chaque bataille se joue sous vos yeux ; l'année s'achève sur la dernière.",
        )
    } else if ready {
        (
            "Vous êtes prêt",
            "En attente des autres seigneurs ; les armées marcheront quand tous auront donné leurs ordres.",
        )
    } else {
        (
            "Guerre",
            "Chaque seigneur donne ses ordres ; les armées marcheront quand tous seront prêts.",
        )
    };
    let mut body = vec![Stack::column()
        .gap(Gap::Sm)
        .children([
            Text::new()
                .variant(TextVariant::Heading3)
                .content(title)
                .build(),
            Text::caption(caption).muted().build(),
        ])
        .build()];
    if let Some(list) = me.filter(|_| ready).map(|id| orders(t, id, false)) {
        body.push(list);
    }
    body.push(section("Royaumes", el(El::Div).append(rows)));
    body.push(latest_news(t, 3));
    Stack::column().gap(Gap::Md).children(body).build()
}

/// The expeditions `id` has ordered this year, one line each; while the
/// orders are open, each can be struck off.
fn orders(t: T, id: Kingdoms, open: bool) -> ElementBuilder {
    let planned = &t.room.seat(id).planned;
    let rows = planned.iter().enumerate().map(|(i, e)| {
        let foe = e.target.map_or("Barbares", |o| o.name());
        let mut cells = vec![el(El::Span).append([
            el(El::Strong).text(foe),
            txt(&format!(" · {}", hommes(e.soldiers))),
        ])];
        if open {
            cells.push(Button::ghost("Retirer").size(ButtonSize::Sm).on_click(by(
                room::withdraw(),
                t.token,
                t.code,
                &[i as u8],
            )));
        }
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::JustifyBetween,
                St::ItemsCenter,
                St::GapSm,
                St::PyXs,
                St::TextSm,
            ])
            .append(cells)
    });
    let body = if planned.is_empty() {
        Text::caption("Aucune expédition cette année.")
            .muted()
            .build()
    } else {
        el(El::Div).append(rows)
    };
    section("Vos ordres", body)
}

mod campaign;

// ---------------------------------------------------------------------------
// The active player's turn
// ---------------------------------------------------------------------------

fn turn(t: T, id: Kingdoms, sheet_open: bool) -> View {
    let room = t.room;
    let k = room.game.kingdom(id);
    let seat = room.seat(id);
    // The roll keeps its deals' notices under their blocks.
    let notice = match (&seat.notice, seat.notice_spot, sheet_open) {
        (Some(notice), Spot::Top, false) => Some(Alert::info().message(notice.clone()).build()),
        _ => None,
    };
    let (body, action) = match seat.step {
        Step::Chronicle => (chronicle_step(t, k), Some(next("Continuer", t))),
        Step::Season => (season_step(t, k, seat), Some(next("Continuer", t))),
        Step::Report => (report_step(seat), Some(next("Continuer", t))),
        Step::Treasury => (treasury_step(k, seat), Some(next("Continuer", t))),
        Step::Intendance => (intendance_step(t, id), None),
        Step::War => (war_step(t, id), Some(end_turn(t, id))),
    };
    let mut items = vec![stepper(seat.step)];
    items.extend(notice);
    items.push(body);
    View {
        body: Stack::column().gap(Gap::Md).children(items).build(),
        action,
    }
}

fn stepper(step: Step) -> ElementBuilder {
    let mut s = Stepper::new().compact(true);
    for label in Step::LABELS {
        s = s.step(label);
    }
    s.current(step.index()).build()
}

// ---------------------------------------------------------------------------
// Saison & Peuple: full-screen animated reports (pure CSS cascades; the whole
// view is the "continue" button)
// ---------------------------------------------------------------------------

/// Cascade slots, .3s apart. A row at slot `i` fades in at `DELAYS[i]`.
const DELAYS: [St; 12] = [
    St::Delay1,
    St::Delay2,
    St::Delay3,
    St::Delay4,
    St::Delay5,
    St::Delay6,
    St::Delay7,
    St::Delay8,
    St::Delay9,
    St::Delay10,
    St::Delay11,
    St::Delay12,
];

/// Slots the Chronique's lines may take before the season follows; further
/// lines appear with the last one.
const NEWS_SLOTS: usize = 4;

/// Slots a count-up occupies before its formatted value takes over (1.2s).
const COUNT_SLOTS: usize = 4;

fn delay(slot: usize) -> St {
    DELAYS[slot.min(DELAYS.len() - 1)]
}

/// The same delay as `delay(slot)`, as a CSS time for inline `--d`.
fn delay_secs(slot: usize) -> String {
    format!("{:.1}s", (slot.min(DELAYS.len() - 1) + 1) as f32 * 0.3)
}

/// Wrap `inner` so it fades up into place at cascade slot `slot`.
fn reveal(slot: usize, inner: ElementBuilder) -> ElementBuilder {
    inner.st([St::AnimateFadeUp, delay(slot)])
}

/// A figure that counts from `from` to `to` over 1.2s starting at `slot`, the
/// raw digits then yielding to the formatted value (`fmt`). Without `@property`
/// support the browser shows the formatted value straight away.
fn count_up(from: i32, to: i32, slot: usize) -> ElementBuilder {
    el(El::Span)
        .st([
            St::PositionRelative,
            St::DisplayInlineBlock,
            St::WhitespaceNowrap,
            St::TabularNums,
        ])
        .append([
            el(El::Span)
                .st([St::AnimateFadeUp, delay(slot + COUNT_SLOTS)])
                .text(&fmt(to)),
            el(El::Span)
                .st([St::PositionAbsolute, St::Inset0, St::CountUp])
                .style(
                    Style::new()
                        .set("--n", &from.to_string())
                        .set("--to", &to.to_string())
                        .set("--d", &delay_secs(slot)),
                ),
        ])
}

/// One ledger line: label left, figure right, optional full-width bar below.
fn ledger_row(
    slot: usize,
    label: ElementBuilder,
    figure: ElementBuilder,
    bar: Option<ElementBuilder>,
) -> ElementBuilder {
    let mut children = vec![
        label.st([St::TextSm]),
        figure.st([St::TabularNums, St::FontSemibold]),
    ];
    if let Some(bar) = bar {
        children.push(bar.st([St::ColSpanFull, St::MtXs]));
    }
    reveal(
        slot,
        el(El::Div)
            .st([
                St::DisplayGrid,
                St::GridColsFrAuto,
                St::ItemsBaseline,
                St::GapSm,
                St::PySm,
                St::BorderB,
            ])
            .append(children),
    )
}

/// Label with a muted aside ("Besoins de l'an · peuple + ost").
fn aside(label: &str, note: &str) -> ElementBuilder {
    el(El::Span).text(label).append([el(El::Span)
        .st([St::TextXs, St::TextMuted, St::MlXs])
        .text(note)])
}

/// Signed figure colored by sign; zero is neutral.
fn signed(n: i32) -> ElementBuilder {
    let (tone, sign) = match n {
        n if n > 0 => (St::TextSuccess, "+"),
        n if n < 0 => (St::TextError, "−"),
        _ => (St::TextMuted, ""),
    };
    el(El::Span)
        .st([tone])
        .text(&format!("{sign}{}", fmt(n.abs())))
}

/// Sky gradient for the year's weather, dark at the foot where the text sits.
fn sky_gradient(weather: Weather) -> &'static str {
    match weather {
        Weather::Great => "linear-gradient(180deg,#f6c453 0%,#e08a3c 55%,#3a2410 100%)",
        Weather::VeryGood => "linear-gradient(180deg,#7fb3d5 0%,#c9b27f 60%,#2f2a1c 100%)",
        Weather::Good => "linear-gradient(180deg,#8b98a6 0%,#5f6b78 60%,#22272d 100%)",
        Weather::Bad => "linear-gradient(180deg,#5a6b7a 0%,#3c4a5a 55%,#161b21 100%)",
        Weather::VeryBad => "linear-gradient(180deg,#2c3e52 0%,#3b4a5a 55%,#1c1e1f 100%)",
        Weather::Disastrous => "linear-gradient(180deg,#c98a3e 0%,#8a5a2b 60%,#2b1a0a 100%)",
    }
}

/// The sun over the year's sky, rising into place: blazing (superbe), veiled
/// (beau), low and pale through the frost (gelées), a heat-haze disk
/// (sécheresse); hidden by the clouds and rain of the middling years.
fn sun(weather: Weather) -> Option<ElementBuilder> {
    let (core, halo, opacity, top) = match weather {
        Weather::Great => ("#fff3c4", "rgba(255,243,196,.35)", St::Opacity100, None),
        Weather::VeryGood => ("#f1d9b2", "rgba(241,217,178,.25)", St::Opacity75, None),
        Weather::VeryBad => (
            "#e8ecf0",
            "rgba(232,236,240,.2)",
            St::Opacity50,
            Some("5.5rem"),
        ),
        Weather::Disastrous => ("#ffb347", "rgba(255,179,71,.4)", St::Opacity75, None),
        Weather::Good | Weather::Bad => return None,
    };
    let mut style = Style::new().background(&format!(
        "radial-gradient(circle,{core} 0 45%,{halo} 60%,transparent 70%)"
    ));
    if let Some(top) = top {
        style = style.set("top", top);
    }
    Some(
        el(El::Div)
            .st([St::GlowDisk, St::AnimateSunUp, opacity])
            .style(style),
    )
}

/// The Saison header: the year's sky and sun, the year and the weather's sentence
/// rising over it.
fn sky(weather: Weather, year: i32, shift: usize) -> ElementBuilder {
    el(El::Div)
        .st([
            St::PositionRelative,
            St::MinH12rem,
            St::RoundedLg,
            St::OverflowHidden,
            St::DisplayFlex,
            St::ItemsEnd,
            St::PMd,
            St::TextOnEmphasis,
        ])
        .style(Style::new().background(sky_gradient(weather)))
        .append(sun(weather))
        .append(precipitation(weather))
        .append([el(El::Div).st([St::PositionRelative]).append([
            reveal(
                shift,
                el(El::Div)
                    .st([St::Text4xl, St::FontBold, St::LeadingNone])
                    .text(&format!("An {year}")),
            ),
            reveal(
                shift + 1,
                el(El::Div)
                    .st([St::TextLg, St::MtSm])
                    .text(weather.sentence()),
            ),
        ])])
}

/// Rain over the flooded year, snow over the early frosts.
fn precipitation(weather: Weather) -> Option<ElementBuilder> {
    let st = match weather {
        Weather::Bad => St::Rain,
        Weather::VeryBad => St::Snow,
        _ => return None,
    };
    Some(el(El::Div).st([st]))
}

/// A ledger's total: ruled above in the accent, no rule below.
fn total_row(slot: usize, label: &str, figure: ElementBuilder) -> ElementBuilder {
    reveal(
        slot,
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::JustifyBetween,
                St::ItemsBaseline,
                St::GapSm,
                St::PySm,
                St::BorderTAccent,
            ])
            .append([
                el(El::Span).st([St::FontMedium]).text(label),
                figure.st([St::TabularNums, St::FontBold]),
            ]),
    )
}

/// A thin bar under a ledger line, growing into place at `slot`.
fn ledger_bar(n: i32, largest: i32, slot: usize) -> ElementBuilder {
    let intent = if n >= 0 {
        ProgressIntent::Success
    } else {
        ProgressIntent::Error
    };
    Progress::new()
        .value(n.unsigned_abs())
        .max(largest.max(1) as u32)
        .intent(intent)
        .thin(true)
        .bar_st([St::AnimateGrow, delay(slot)])
        .build()
}

/// Chronique: the year gone by since this seigneur's council.
fn chronicle_step(t: T, k: &Kingdom) -> ElementBuilder {
    let year = t.room.game.year;
    chronicle(
        t.room,
        k.id,
        format!("An {} · depuis votre conseil", year - 1),
        0,
    )
}

/// The seat's news as a list, one line per fact, fading in from `slot`.
fn chronicle(room: &Room, id: Kingdoms, title: impl Into<Label>, slot: usize) -> ElementBuilder {
    let k = room.game.kingdom(id);
    let news = &room.seat(id).news;
    let last_sale = news.iter().rposition(|n| matches!(n, News::Sold { .. }));
    let lines: Vec<ElementBuilder> = if news.is_empty() {
        vec![news_line(
            Icon::Map,
            Tone::Neutral,
            "Un an sans histoire.",
            "Personne n'a marché sur vos terres, la peste vous a épargné et le marché n'a rien vendu.",
        )]
    } else {
        news.iter()
            .enumerate()
            .map(|(i, n)| tell(room, k, n, last_sale == Some(i)))
            .collect()
    };
    let title = title.into();
    let list = el(El::Ul)
        .st([St::ListStyleNone, St::P0, St::M0])
        .at_str(At::AriaLabel, &format!("Chronique · {title}"))
        .append(
            lines
                .into_iter()
                .enumerate()
                .map(|(i, line)| reveal(slot + i.min(NEWS_SLOTS - 1), line)),
        );
    section(title, list)
}

/// The colour of a Chronique line's icon; the words carry the meaning too.
#[derive(Clone, Copy)]
enum Tone {
    Good,
    Bad,
    Neutral,
    Gold,
}

/// One fact: a tinted icon, the verdict in bold, the figures in a muted line.
fn news_line(icon: Icon, tone: Tone, verdict: &str, detail: &str) -> ElementBuilder {
    let tint = match tone {
        Tone::Good => St::TextSuccess,
        Tone::Bad => St::TextError,
        Tone::Neutral => St::TextMuted,
        Tone::Gold => St::TextAccent,
    };
    let mut text = vec![el(El::Div).st([St::FontSemibold]).text(verdict)];
    if !detail.is_empty() {
        text.push(
            el(El::Div)
                .st([St::TextSm, St::TextMuted, St::TabularNums])
                .text(detail),
        );
    }
    el(El::Li)
        .st([
            St::DisplayFlex,
            St::ItemsStart,
            St::GapSm,
            St::PySm,
            St::BorderB,
        ])
        .append([
            icon_sized(icon, 20)
                .st([tint, St::FlexShrink0, St::MtXs])
                .at(At::AriaHidden, Av::True),
            el(El::Div).st([St::MinW0]).append(text),
        ])
}

/// Put one fact of `k`'s year into words. `last_sale` marks the sale after
/// which what stayed unsold is worth telling.
fn tell(room: &Room, k: &Kingdom, news: &News, last_sale: bool) -> ElementBuilder {
    match news {
        News::Sold {
            buyer,
            amount,
            price,
            left,
        } => {
            let mut detail = format!(
                "+{} au trésor",
                coins(grain_value(*amount, *price), k.currency())
            );
            if last_sale {
                detail.push_str(&if *left > 0 {
                    format!(" · {} restent au marché à {price}", fmt(*left))
                } else {
                    " · tout est vendu".to_string()
                });
            }
            news_line(
                Icon::Scale,
                Tone::Good,
                &format!("{} boisseaux vendus à la {}", fmt(*amount), buyer.name()),
                &detail,
            )
        }
        News::Expeditions {
            count,
            spoils,
            men_lost,
            wiped,
        } => {
            let verdict = if *count == 1 {
                "Votre expédition".to_string()
            } else {
                format!("Vos expéditions : {count}")
            };
            let mut parts = vec![if spoils.arpents > 0 {
                format!("{} arpents conquis", fmt(spoils.arpents))
            } else {
                "aucun arpent conquis".to_string()
            }];
            parts.extend(goods_fr(spoils, k.currency()));
            parts.extend(people_fr(spoils, Side::Attacker));
            parts.extend(buildings_fr(spoils));
            parts.push(format!("{} perdus", hommes(*men_lost)));
            match *wiped {
                0 => {}
                1 => parts.push("une expédition anéantie".to_string()),
                n => parts.push(format!("{n} expéditions anéanties")),
            }
            let tone = if *wiped > 0 {
                Tone::Bad
            } else if spoils.arpents > 0 {
                Tone::Good
            } else {
                Tone::Neutral
            };
            news_line(Icon::Swords, tone, &verdict, &parts.join(" · "))
        }
        News::Attacked {
            armies,
            repelled,
            levy,
            garrison_fallen,
            spoils,
        } => {
            let names: Vec<String> = armies
                .iter()
                .map(|(by, _)| format!("la {}", by.name()))
                .collect();
            let men: i32 = armies.iter().map(|(_, n)| n).sum();
            let title = format!(
                "{} {} marché sur vous avec {}",
                names.join(" et "),
                if names.len() > 1 { "ont" } else { "a" },
                hommes(men)
            );
            let title = title[..1].to_uppercase() + &title[1..];
            let mut parts = Vec::new();
            if *repelled {
                parts.push(if *levy {
                    "Repoussée par le peuple en armes".to_string()
                } else {
                    "Repoussée · la terre est intacte".to_string()
                });
            } else {
                parts.push(format!("−{} arpents", fmt(spoils.arpents)));
                if *levy {
                    parts.push("sans garnison, le peuple a pris les armes".to_string());
                } else {
                    parts.push("la garnison est balayée".to_string());
                }
            }
            if *garrison_fallen > 0 {
                parts.push(format!("{} d'armes tombés", hommes(*garrison_fallen)));
            }
            if !*repelled {
                parts.extend(
                    goods_fr(spoils, k.currency())
                        .into_iter()
                        .map(|g| format!("−{g}")),
                );
                parts.extend(people_fr(spoils, Side::Defender));
                parts.extend(buildings_fr(spoils));
            }
            news_line(
                Icon::Swords,
                if *repelled { Tone::Good } else { Tone::Bad },
                &title,
                &parts.join(" · "),
            )
        }
        News::Plague(p) => {
            let classes = [
                (p.serfs_killed, "serfs"),
                (p.merchants_killed, "marchands"),
                (p.soldiers_killed, "hommes d'armes"),
                (p.nobles_killed, "nobles"),
            ];
            let total: i32 = classes.iter().map(|(n, _)| n).sum();
            let detail: Vec<String> = classes
                .into_iter()
                .filter(|(n, _)| *n > 0)
                .map(|(n, what)| format!("{} {what}", fmt(n)))
                .collect();
            news_line(
                Icon::Skull,
                Tone::Bad,
                &format!("La peste a ravagé la {}", k.name()),
                &format!("{} morts : {}", fmt(total), detail.join(", ")),
            )
        }
        News::Rank { before, now } => {
            let title = k.id.title_name(*now);
            if now > before {
                news_line(
                    rank_icon(*now),
                    Tone::Gold,
                    &format!("{} de {} est fait {title}", k.player_name, k.name()),
                    "Touchez la marque du header pour la fiche du titre",
                )
            } else {
                news_line(
                    rank_icon(*now),
                    Tone::Bad,
                    &format!("{} de {} n'est plus que {title}", k.player_name, k.name()),
                    "Un critère du titre n'est plus rempli — voir la fiche du titre",
                )
            }
        }
        News::Elsewhere(fact) => {
            let line = match *fact {
                Elsewhere::RulerDied { id, cause } => {
                    let o = room.game.kingdom(id);
                    format!(
                        "Ailleurs : {} {} ; la {} se disloque.",
                        o.full_title(),
                        room::death_fr(&cause),
                        o.name()
                    )
                }
                Elsewhere::Annexed { id, by } => {
                    format!(
                        "Ailleurs : la {} est annexée par la {}.",
                        id.name(),
                        by.name()
                    )
                }
                Elsewhere::Rank { id, before, now } => {
                    let o = room.game.kingdom(id);
                    let verb = if now > before {
                        "est fait"
                    } else {
                        "n'est plus que"
                    };
                    format!(
                        "Ailleurs : {} de {} {verb} {}.",
                        o.player_name,
                        o.name(),
                        id.title_name(now)
                    )
                }
                Elsewhere::Crowned(id) => {
                    let o = room.game.kingdom(id);
                    format!(
                        "Ailleurs : {} de {} est couronné {}.",
                        o.player_name,
                        o.name(),
                        id.title_name(PlayerTitle::Emperor)
                    )
                }
            };
            news_line(Icon::Map, Tone::Neutral, &line, "")
        }
        News::Fallen(fate) => match *fate {
            Fate::Annexed(by) => {
                let b = room.game.kingdom(by);
                news_line(
                    Icon::BrokenBanner,
                    Tone::Bad,
                    &format!(
                        "Les armées de {} ont pris vos dernières terres",
                        b.full_title()
                    ),
                    &format!(
                        "La {} est annexée · vos serfs jurent fidélité à {}",
                        k.name(),
                        b.titled_name()
                    ),
                )
            }
            Fate::RulerDied(cause) => news_line(
                Icon::BrokenBanner,
                Tone::Bad,
                &format!("{} {}", k.player_name, room::death_fr(&cause)),
                &format!("La {} se disloque", k.name()),
            ),
        },
        News::Crowned => news_line(
            Icon::ImperialCrown,
            Tone::Gold,
            "Six royaumes, un seul empereur",
            &format!(
                "{} de {} est couronné {}",
                k.player_name,
                k.name(),
                k.id.title_name(PlayerTitle::Emperor)
            ),
        ),
    }
}

/// Saison: the year's sky, then the grain ledger it leaves — read before
/// the market opens, so the granaries hold last year's stocks and the harvest.
fn season_step(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let game = &t.room.game;
    let shift = 0;
    let sky = sky(game.weather, game.year, shift);

    let needs = k.peasants_grain_needs() + k.soldiers_grain_needs();
    let balance = k.grain_stocks - needs;
    let dawn = seat.stocks_at_dawn;
    let before = dawn + seat.rats - k.grain_harvest;
    let largest = k.grain_harvest.max(dawn).max(needs);
    let (verdict, tone) = if balance < 0 {
        ("Il manque", St::TextError)
    } else {
        ("Il reste", St::TextSuccess)
    };
    let ledger = el(El::Div).st([St::PxXs]).append([
        ledger_row(
            shift + 2,
            el(El::Span).text("Réserves de l'an passé"),
            el(El::Span).st([St::TextMuted]).text(&fmt(before)),
            None,
        ),
        ledger_row(
            shift + 2,
            el(El::Span).text("Récolte"),
            el(El::Span)
                .st([St::TextSuccess])
                .append([txt("+"), count_up(0, k.grain_harvest, shift + 2)]),
            Some(ledger_bar(k.grain_harvest, largest, shift + 2)),
        ),
        ledger_row(
            shift + COUNT_SLOTS,
            aside("Mangés par les rats", &format!("· {} %", k.rats_loss_rate)),
            signed(-seat.rats),
            Some(ledger_bar(-seat.rats, largest, shift + COUNT_SLOTS)),
        ),
        total_row(shift + 5, "Réserves", el(El::Span).text(&fmt(dawn))),
    ]);
    let ledger = ledger.append([
        ledger_row(
            shift + 6,
            aside("Besoins de l'an", "· peuple + ost"),
            signed(-needs),
            Some(ledger_bar(-needs, largest, shift + 6)),
        ),
        total_row(
            shift + 7,
            verdict,
            el(El::Span)
                .st([tone, St::TextLg])
                .text(&format!("{} boisseaux", fmt(balance.abs()))),
        ),
    ]);
    Stack::column().gap(Gap::Lg).children([sky, ledger]).build()
}

// ---------------------------------------------------------------------------
// The market sheets: buy from a stall, list grain, sell land. Each is one
// form — the explanation, the figures following the thumb, the slider, the
// verb — concluded by `room::trade`.
// ---------------------------------------------------------------------------

/// The muted line under a sheet's title.
fn subtitle(parts: Vec<ElementBuilder>) -> ElementBuilder {
    el(El::Div)
        .st([St::TextSm, St::TextMuted, St::TabularNums])
        .append(parts)
}

/// What the thing does, in two sentences of game French.
fn intro(text: &str) -> ElementBuilder {
    el(El::P).st([St::TextSm, St::M0]).text(text)
}

/// A line of a sheet: `label` on the left, a figure on the right.
fn line(label: &str, figure: Vec<ElementBuilder>) -> ElementBuilder {
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::GapSm,
            St::TextSm,
            St::PyXs,
            St::BorderB,
        ])
        .append([
            el(El::Span).st([St::TextMuted]).text(label),
            el(El::Span)
                .st([St::TabularNums, St::FontSemibold])
                .append(figure),
        ])
}

/// The verb that concludes a sheet, its label following the sliders.
fn cta(parts: Vec<ElementBuilder>) -> ElementBuilder {
    Button::new()
        .intent(ButtonIntent::Primary)
        .size(ButtonSize::Lg)
        .full_width(true)
        .build()
        .append([el(El::Span).append(parts)])
}

/// The verb of a sheet that has nothing to conclude, and why.
fn refused(reason: &str, label: impl Into<Label>) -> [ElementBuilder; 2] {
    [
        el(El::P)
            .st([St::TextSm, St::TextWarning, St::M0, St::PtSm])
            .text(reason),
        Button::primary(label)
            .size(ButtonSize::Lg)
            .full_width(true)
            .disabled(true)
            .build(),
    ]
}

/// `base + f(n)` along a slider from `min` to `max` standing at `value`,
/// interpolated between its ends.
fn along(ch: u16, value: i32, min: i32, max: i32, f: &dyn Fn(i32) -> i32) -> LiveSum {
    LiveSum {
        base: f(value),
        terms: vec![(ch, vec![f(min) - f(value), f(max) - f(value)])],
    }
}

/// The trade slider: released values round-trip so the live figures re-centre.
fn deal_slider(t: T, which: u8, ch: u16, label: impl Into<Label>) -> Slider {
    let name = if which == 0 { "amount" } else { "price" };
    Slider::new()
        .name(name)
        .id(name)
        .channel(ch)
        .grouped(fmt)
        .label(label)
        .on_change(by(room::deal_draft(), t.token, t.code, &[which]))
}

/// What the treasury can buy from `o`, capped by what is on the market.
fn can_buy(k: &Kingdom, s: &Kingdom) -> i32 {
    let price = s.grain_price.min(MAX_GRAIN_PRICE);
    if price < 1 {
        return 0;
    }
    (k.treasury * 90 / price).min(s.grain_to_sell).max(0)
}

/// Buy from `o`: the grain and the courtage, what the treasury and the
/// granaries hold after, following the thumb.
fn buy_sheet(t: T, k: &Kingdom, seat: &Seat, o: Kingdoms) -> ElementBuilder {
    let s = t.room.game.kingdom(o);
    let price = s.grain_price.min(MAX_GRAIN_PRICE);
    let cur = k.currency();
    let max = can_buy(k, s);
    let cost = |n: i32| calculate_buy_cost(n, price);
    let mut body = vec![
        subtitle(vec![txt(&format!(
            "{price} le cent · {} bx en vente · courtage 10 %",
            fmt(s.grain_to_sell)
        ))]),
        intro(&format!(
            "Le grain arrive aux greniers tout de suite ; la {} en fera état dans sa chronique. \
             Le courtage revient aux marchands du port.",
            o.name()
        )),
    ];
    if max < 1 {
        let reason = if s.grain_to_sell < 1 || price < 1 {
            format!("La {} n'a plus de grain à vendre.", o.name())
        } else {
            format!(
                "Il manque {} pour le premier cent.",
                coins(cost(100).max(1) - k.treasury, cur)
            )
        };
        body.extend(refused(&reason, "Acheter du grain"));
        return Stack::column().gap(Gap::Sm).children(body).build();
    }
    let amount = seat.deal_amount.unwrap_or((max / 10).max(1)).clamp(1, max);
    let ch = rwire::builder::next_live_channel();
    let live = |f: &dyn Fn(i32) -> i32| {
        el(El::Span)
            .text(&fmt(f(amount)))
            .live_sum(along(ch, amount, 1, max, f))
    };
    let live_signed = |f: &dyn Fn(i32) -> i32| {
        el(El::Span)
            .text(&delta(f(amount)))
            .live_sum_signed(along(ch, amount, 1, max, f))
    };
    let word = |f: &dyn Fn(i32) -> i32| live_coin_word(ch, cur, amount, 1, max, f);
    let grain = |n: i32| -grain_value(n, price);
    let brokerage = |n: i32| grain_value(n, price) - cost(n);
    let after = |n: i32| k.treasury - cost(n);
    body.extend([
        el(El::Div).append([
            line("Grain", vec![live_signed(&grain), txt(" "), word(&grain)]),
            line(
                "Courtage",
                vec![live_signed(&brokerage), txt(" "), word(&brokerage)],
            ),
            line("Trésor après", vec![live(&after), txt(" "), word(&after)]),
            line(
                "Réserves après",
                vec![live(&|n| k.grain_stocks + n), txt(" bx")],
            ),
        ]),
        deal_slider(t, 0, ch, "Boisseaux")
            .min(1)
            .max(max)
            .value(amount)
            .readout_suffix(
                el(El::Span)
                    .st([St::TextSm, St::TextMuted, St::TabularNums])
                    .append([
                        txt("· "),
                        live(&cost),
                        txt(" "),
                        word(&cost),
                        txt(&format!(" · max {} bx", fmt(max))),
                    ]),
            )
            .build(),
        cta(vec![
            txt("Acheter "),
            el(El::Span).text(&fmt(amount)).live_text_grouped(ch),
            txt(" boisseaux"),
        ]),
    ]);
    form(
        by(room::trade(), t.token, t.code, &[Deal::Buy(o).code()]),
        body,
    )
}

/// List grain: amount and price the hundred, what it brings if it all sells,
/// what stays in the granaries and whether the year's bread is still covered.
fn sell_sheet(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let stocks = k.grain_stocks;
    let cur = k.currency();
    let surplus = stocks - k.peasants_grain_needs() - k.soldiers_grain_needs();
    let mut body = vec![
        subtitle(vec![txt(&format!(
            "réserves {} bx · {} {} bx",
            fmt(stocks),
            if surplus < 0 { "manque" } else { "surplus" },
            fmt(surplus.abs())
        ))]),
        intro(
            "Le lot part au marché l'an prochain ; il se vend au fil de l'année, et chaque acheteur \
             vous paie comptant. Ce qui ne se vend pas revient aux greniers. Au-delà du surplus, \
             vous vendez le pain de vos sujets.",
        ),
    ];
    if stocks < 1 {
        body.extend(refused("Les greniers sont vides.", "Mettre en vente"));
        return Stack::column().gap(Gap::Sm).children(body).build();
    }
    let amount = seat
        .deal_amount
        .unwrap_or(surplus.clamp(1, stocks))
        .clamp(1, stocks);
    let price = seat
        .deal_price
        .unwrap_or(CUSTOMARY_PRICE)
        .clamp(1, MAX_GRAIN_PRICE);
    let ch_a = rwire::builder::next_live_channel();
    let ch_p = rwire::builder::next_live_channel();
    let value = |a: i32, p: i32| grain_value(a, p);
    let now = value(amount, price);
    let brings = LiveSum {
        base: now,
        terms: vec![
            (
                ch_a,
                vec![value(1, price) - now, value(stocks, price) - now],
            ),
            (
                ch_p,
                vec![value(amount, 1) - now, value(amount, MAX_GRAIN_PRICE) - now],
            ),
        ],
    };
    // Covered while the lot stays within the surplus: the two answers swap
    // as the thumb crosses it.
    let covered = el(El::Span)
        .live_switch(ch_a, &[(surplus.max(0) + 1) as u32])
        .append([
            hidden_unless(
                amount <= surplus,
                el(El::Span).st([St::TextSuccess]).append([
                    txt("oui · "),
                    el(El::Span)
                        .text(&fmt(surplus - amount))
                        .live_remainder_grouped(surplus.max(0) as u32, &[ch_a]),
                    txt(" de marge"),
                ]),
            ),
            hidden_unless(
                amount > surplus,
                el(El::Span).st([St::TextError]).append([
                    txt("non · il manquera "),
                    el(El::Span).text(&fmt(amount - surplus)).live_sum(along(
                        ch_a,
                        amount,
                        1,
                        stocks,
                        &|n| n - surplus,
                    )),
                    txt(" bx"),
                ]),
            ),
        ]);
    let mut lines = Vec::new();
    // What is already listed this year: a second lot joins it (averaged
    // price), so the figures below are the new lot alone.
    if let Some((a, p)) = k.listing {
        lines.push(line(
            "Déjà en vente cette année",
            vec![txt(&format!(
                "{} bx à {p} · {}",
                fmt(a),
                coins(value(a, p), cur)
            ))],
        ));
    }
    lines.extend([
        line(
            "Si tout se vend",
            vec![
                el(El::Span).text(&delta(now)).live_sum_signed(brings),
                txt(" "),
                live_coin_word(ch_a, cur, amount, 1, stocks, &|a| value(a, price)),
            ],
        ),
        line(
            "Réserves après",
            vec![
                el(El::Span)
                    .text(&fmt(stocks - amount))
                    .live_remainder_grouped(stocks as u32, &[ch_a]),
                txt(" bx"),
            ],
        ),
        line("Besoins de l'an couverts", vec![covered]),
    ]);
    let mut amount_slider = deal_slider(t, 0, ch_a, "Boisseaux à vendre")
        .min(1)
        .max(stocks)
        .value(amount);
    if surplus > 0 && surplus < stocks {
        amount_slider = amount_slider.mark(surplus, "surplus");
    }
    let verb = if k.listing.is_some() {
        "Ajouter "
    } else {
        "Mettre "
    };
    body.extend([
        el(El::Div).append(lines),
        amount_slider.build(),
        deal_slider(t, 1, ch_p, "Prix du cent")
            .min(1)
            .max(MAX_GRAIN_PRICE)
            .value(price)
            .unit(cur)
            .build(),
        cta(vec![
            txt(verb),
            el(El::Span).text(&fmt(amount)).live_text_grouped(ch_a),
            txt(" boisseaux en vente à "),
            el(El::Span).text(&price.to_string()).live_text(ch_p),
        ]),
    ]);
    form(
        by(room::trade(), t.token, t.code, &[Deal::Sell.code()]),
        body,
    )
}

/// Sell land to the barbarians: the arpents, the recette, the surface after
/// and the harvest each of them would have given.
fn land_sheet(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let cur = k.currency();
    let max = max_land_sale(k.surface);
    let mut body = vec![
        subtitle(vec![txt(&format!(
            "{} arpents · {} au plus cette année",
            fmt(k.surface),
            fmt(max)
        ))]),
        intro(&format!(
            "Les Barbares paient {LAND_SELL_PRICE} {cur} l'arpent et gardent ce qu'ils achètent. \
             Une terre vendue ne rapporte plus de grain : par beau temps, chaque arpent en donne \
             ~3,6 bx l'an."
        )),
    ];
    if max < 1 {
        body.extend(refused(
            "Il ne reste pas assez de terres à vendre.",
            "Vendre des terres",
        ));
        return Stack::column().gap(Gap::Sm).children(body).build();
    }
    let amount = seat
        .deal_amount
        .unwrap_or((k.surface / 100).max(1))
        .clamp(1, max);
    let ch = rwire::builder::next_live_channel();
    let gain = |e: ElementBuilder| e.live_scaled_grouped(ch, LAND_SELL_PRICE as u32, 1);
    body.extend([
        el(El::Div).append([
            line(
                "Recette",
                vec![
                    txt("+"),
                    gain(el(El::Span).text(&fmt(amount * LAND_SELL_PRICE))),
                    txt(&format!(" {cur}")),
                ],
            ),
            line(
                "Surface après",
                vec![
                    el(El::Span)
                        .text(&fmt(k.surface - amount))
                        .live_remainder_grouped(k.surface as u32, &[ch]),
                    txt(" arpents"),
                ],
            ),
            line(
                "Récolte, chaque an",
                vec![
                    txt("≈ −"),
                    el(El::Span)
                        .text(&fmt(amount * 36 / 10))
                        .live_scaled_grouped(ch, 36, 10),
                    txt(" bx"),
                ],
            ),
        ]),
        deal_slider(t, 0, ch, "Arpents aux Barbares")
            .min(1)
            .max(max)
            .value(amount)
            .readout_suffix(
                el(El::Span)
                    .st([St::TextSm, St::TextMuted, St::TabularNums])
                    .text(&format!("· max {}", fmt(max))),
            )
            .build(),
        cta(vec![
            txt("Vendre "),
            el(El::Span).text(&fmt(amount)).live_text_grouped(ch),
            txt(" arpents"),
        ]),
    ]);
    form(
        by(room::trade(), t.token, t.code, &[Deal::Land.code()]),
        body,
    )
}

/// Every kingdom currently selling grain.
/// The market's going rate, the cent: the stalls' mean asking price, or the
/// customary 20 when nobody sells.
const CUSTOMARY_PRICE: i32 = 20;
fn going_rate(room: &Room) -> i32 {
    let stalls = sellers(room);
    if stalls.is_empty() {
        return CUSTOMARY_PRICE;
    }
    let total: i32 = stalls
        .iter()
        .map(|&o| room.game.kingdom(o).grain_price.min(MAX_GRAIN_PRICE))
        .sum();
    total / stalls.len() as i32
}

fn sellers(room: &Room) -> Vec<Kingdoms> {
    room.game
        .alive_kingdoms()
        .into_iter()
        .filter(|&o| {
            let s = room.game.kingdom(o);
            s.grain_to_sell > 0 && s.grain_price > 0
        })
        .collect()
}

/// Every current listing on the grain market, the viewer's own marked.
fn market_section(t: T, viewer: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
    let listed = sellers(room);
    if listed.is_empty() {
        return section(
            "Marché du grain",
            Text::body("Personne ne vend de grain en ce moment.")
                .muted()
                .build(),
        );
    }
    let rows = listed.into_iter().map(|o| {
        let s = room.game.kingdom(o);
        let offer = format!(
            "{} bx à {} le cent",
            fmt(s.grain_to_sell),
            fmt(s.grain_price.min(MAX_GRAIN_PRICE))
        );
        let who = el(El::Div).append([
            el(El::Strong).text(s.name()),
            el(El::Span)
                .st([St::TextSm, St::TextMuted, St::MlXs, St::TabularNums])
                .text(&offer),
        ]);
        let mut row = vec![who];
        if viewer == Some(o) {
            row.push(Badge::new().text("vous").build());
        }
        Stack::row()
            .align_center()
            .justify(StackJustify::Between)
            .gap(Gap::Sm)
            .children(row)
            .build()
            .st([St::PySm, St::BorderB])
    });
    section("Marché du grain", el(El::Div).append(rows))
}

fn rename_form(t: T, k: &Kingdom) -> ElementBuilder {
    form(
        t.act(room::rename()),
        [
            Input::text()
                .name("name")
                .id("name")
                .value(k.player_name.clone())
                .placeholder("Nom du seigneur")
                .required(true)
                .build(),
            Button::primary("Prendre ce nom").full_width(true).build(),
        ],
    )
}

/// The title sheet: the rank held, the next one, and every criterion of the
/// next rank with its gauge. Rendered live, so it follows the purchases.
fn title_sheet(t: T, k: &Kingdom) -> ElementBuilder {
    let seat = t.room.seat(k.id);
    let since = if seat.title_year > 0 {
        format!("Depuis l'an {}", seat.title_year)
    } else {
        "Titre de départ".to_string()
    };
    let head = el(El::Div)
        .st([St::DisplayFlex, St::ItemsCenter, St::GapMd])
        .append([
            rank_mark(k, 36, St::TextDefault),
            el(El::Div).st([St::MinW0]).append([
                el(El::Strong)
                    .st([St::DisplayBlock, St::TextLg])
                    .text(&k.full_title()),
                Text::caption(since).muted().build(),
            ]),
        ]);
    let mut body = vec![head];
    match k.title().next() {
        Some(next) => {
            let progress = k.progress(next);
            let met = progress.iter().filter(|c| c.met()).count();
            body.push(
                el(El::Div)
                    .st([
                        St::DisplayFlex,
                        St::JustifyBetween,
                        St::ItemsBaseline,
                        St::GapSm,
                        St::TextSm,
                    ])
                    .append([
                        el(El::Span).st([St::TextMuted]).text("Prochain titre"),
                        el(El::Strong).text(&format!(
                            "{} de {} · {met} critère{} sur {}",
                            k.id.title_name(next),
                            k.name(),
                            if met > 1 { "s" } else { "" },
                            progress.len()
                        )),
                    ]),
            );
            body.extend(progress.iter().map(criterion_row));
        }
        None => body.push(
            Text::body("Titre suprême : il n'y a plus rien au-dessus, seulement à le garder.")
                .muted()
                .build(),
        ),
    }
    body.push(
        Text::caption("Le titre se perd si un critère cesse d'être rempli.")
            .muted()
            .build(),
    );
    Stack::column().gap(Gap::Sm).children(body).build()
}

/// One criterion: label, "have / need" with a check when met, and a thin gauge.
fn criterion_row(c: &Criterion) -> ElementBuilder {
    let (label, figure): (&str, fn(i32) -> String) = match c.what {
        Requirement::Peasants => ("Serfs", fmt),
        Requirement::LandRatio => ("Arpents par serf", tenths),
        Requirement::Nobles => ("Nobles", fmt),
        Requirement::GrainMills => ("Moulins", fmt),
        Requirement::Marketplaces => ("Champs de foire", fmt),
        Requirement::Foundries => ("Fonderie", fmt),
        Requirement::Palaces => ("Palais", |n| format!("{} %", n * 10)),
    };
    let met = c.met();
    let figures = el(El::Span)
        .st([
            St::TabularNums,
            St::WhitespaceNowrap,
            if met { St::TextSuccess } else { St::TextMuted },
        ])
        .text(&format!(
            "{} / {}{}",
            figure(c.have),
            figure(c.need),
            if met { " ✓" } else { "" }
        ));
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapXs, St::TextSm])
        .append([
            el(El::Div)
                .st([St::DisplayFlex, St::JustifyBetween, St::ItemsBaseline])
                .append([el(El::Span).text(label), figures]),
            Progress::new()
                .value(c.have.clamp(0, c.need) as u32)
                .max(c.need.max(1) as u32)
                .intent(if met {
                    ProgressIntent::Success
                } else {
                    ProgressIntent::Primary
                })
                .thin(true)
                .build(),
        ])
}

// ---------------------------------------------------------------------------
// Le prévisionnel: the year to come in six figures — peuple, nobles,
// marchands, ost, trésor, réserve. Every figure is `now + Σ (along each
// slider − now)` with the tables centred on the current settings, so it
// follows the thumb client-side and is exact while one thumb moves; a sheet
// validated round-trips and the whole roll re-centres.
// ---------------------------------------------------------------------------

/// The six cells of the register, plus what the council's sliders read.
#[derive(Clone, Copy)]
struct Prospect {
    people: Outlook,
    nobles: Outlook,
    merchants: Outlook,
    efficiency: i32,
    losses: Outlook,
    soldiers: i32,
    net: Outlook,
    customs: Outlook,
    sales: Outlook,
    income: Outlook,
    reserve: i32,
}

/// The year to come for `k` under `draft`, told as changes from `base` — the
/// kingdom as it stands — so a purchase's immediate moves (the merchants a
/// fair draws, the nobles a palace draws, the recruits) count with the year's.
fn prospect(base: &Kingdom, k: &Kingdom, weather: Weather, draft: Draft) -> Prospect {
    let council = draft.council();
    let demo = demography_outlook(k, council);
    let eco = economy_outlook(k, weather, demo.immigrants, council.taxes);
    let moved = |a: i32, b: i32| Outlook::sure(b - a);
    Prospect {
        people: demo.people + moved(base.population(), k.population()),
        nobles: demo.nobles + moved(base.nobles, k.nobles),
        merchants: demo.merchants + moved(base.merchants, k.merchants),
        efficiency: demo.soldiers_efficiency,
        losses: demo.army_losses,
        soldiers: k.soldiers,
        net: eco.net(),
        customs: eco.immigration_taxes,
        sales: eco.commercial_taxes,
        income: Outlook::sure(eco.income_taxes),
        reserve: k.grain_stocks - council.grain_for_peasants(k) - council.grain_for_soldiers(k),
    }
}

impl Prospect {
    /// The wider of two prospects — a purchase whose side effect is a dice
    /// roll is told between its worst and best draw.
    fn merge(a: Prospect, b: Prospect) -> Prospect {
        let o = |x: Outlook, y: Outlook| Outlook {
            low: x.low.min(y.low),
            expected: (x.expected + y.expected) / 2,
            high: x.high.max(y.high),
        };
        Prospect {
            people: o(a.people, b.people),
            nobles: o(a.nobles, b.nobles),
            merchants: o(a.merchants, b.merchants),
            efficiency: (a.efficiency + b.efficiency) / 2,
            losses: o(a.losses, b.losses),
            soldiers: a.soldiers,
            net: o(a.net, b.net),
            customs: o(a.customs, b.customs),
            sales: o(a.sales, b.sales),
            income: o(a.income, b.income),
            reserve: (a.reserve + b.reserve) / 2,
        }
    }
}

/// What a sheet's slider runs along: a council field, or the units of a
/// purchase.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Along {
    Field(Field),
    Units(InvestmentType),
}

/// One slider: its channel, run and the prospects along it, evenly spaced
/// over its run (at most 121 points; `live_lookup` interpolates).
struct Axis {
    along: Along,
    channel: u16,
    max: i32,
    curve: Vec<Prospect>,
}

/// The values a slider is sampled at: every integer of a short run, 121
/// evenly spaced points of a long one.
fn samples(min: i32, max: i32) -> Vec<i32> {
    let n = max - min + 1;
    if n <= 121 {
        (min..=max).collect()
    } else {
        (0..121).map(|i| min + (max - min) * i / 120).collect()
    }
}

/// The register's table: the kingdom, the drafts, the prospect before and
/// after the sheet's own move, and the axis its slider runs along (none for
/// the roll).
struct Board<'a> {
    k: &'a Kingdom,
    draft: Draft,
    before: Prospect,
    now: Prospect,
    axes: Vec<Axis>,
}

impl Board<'_> {
    /// The roll: the year as it is settled, no slider.
    fn council(k: &Kingdom, weather: Weather, draft: Draft) -> Board<'_> {
        let now = prospect(k, k, weather, draft);
        Board {
            k,
            draft,
            before: now,
            now,
            axes: Vec::new(),
        }
    }

    /// A council sheet: `field` runs its whole range, the other four held.
    fn along(k: &Kingdom, weather: Weather, draft: Draft, field: Field) -> Board<'_> {
        let (peasants_max, soldiers_max) = Draft::bounds(k);
        let max = match field {
            Field::Peasants => peasants_max,
            Field::Soldiers => soldiers_max,
            Field::Customs => Taxes::MAX_CUSTOMS,
            Field::Sales => Taxes::MAX_SALES,
            Field::Income => Taxes::MAX_INCOME,
        };
        let curve = (0..=max)
            .map(|v| {
                let mut d = draft;
                d.set(field, v);
                prospect(k, k, weather, d)
            })
            .collect();
        let mut b = Board::council(k, weather, draft);
        b.axes.push(Axis {
            along: Along::Field(field),
            channel: rwire::builder::next_live_channel(),
            max,
            curve,
        });
        b
    }

    /// A purchase sheet: `now` is the year with one unit bought, the axis
    /// runs from one to `max` units (none when the treasury can't buy one).
    fn purchase(
        k: &Kingdom,
        weather: Weather,
        draft: Draft,
        kind: InvestmentType,
        max: i32,
    ) -> Board<'_> {
        let at = |n: i32| {
            let (lo, hi) = bought(k, kind, n);
            Prospect::merge(
                prospect(k, &lo, weather, draft),
                prospect(k, &hi, weather, draft),
            )
        };
        let mut b = Board::council(k, weather, draft);
        b.now = at(1);
        if max >= 1 {
            b.axes.push(Axis {
                along: Along::Units(kind),
                channel: rwire::builder::next_live_channel(),
                max,
                curve: samples(1, max).into_iter().map(at).collect(),
            });
        }
        b
    }

    fn find(&self, along: Along) -> Option<&Axis> {
        self.axes.iter().find(|a| a.along == along)
    }

    /// The sheet's own slider.
    fn slider(&self) -> &Axis {
        &self.axes[0]
    }

    /// The figure now plus its deviation along every slider it depends on.
    fn sum(&self, figure: impl Fn(&Prospect) -> i32) -> LiveSum {
        let base = figure(&self.now);
        let terms = self
            .axes
            .iter()
            .filter_map(|a| {
                let table: Vec<i32> = a.curve.iter().map(|p| figure(p) - base).collect();
                table.iter().any(|&d| d != 0).then_some((a.channel, table))
            })
            .collect();
        LiveSum { base, terms }
    }

    /// A bounded figure: its bounds now and the sums that follow the thumbs.
    fn bounds(&self, figure: impl Fn(&Prospect) -> Outlook) -> Bounds {
        Bounds {
            now: figure(&self.now),
            low: self.sum(|p| figure(p).low),
            high: self.sum(|p| figure(p).high),
        }
    }
}

/// The kingdom with `n` units of `kind` bought, at the worst and best draw
/// of the dice a fair or a palace rolls.
fn bought(k: &Kingdom, kind: InvestmentType, n: i32) -> (Kingdom, Kingdom) {
    let mut lo = k.clone();
    lo.treasury -= n * kind.cost();
    let mut hi = lo.clone();
    match kind {
        InvestmentType::Marketplaces => {
            lo.marketplaces += n;
            hi.marketplaces += n;
            lo.merchants += n;
            lo.peasants -= n;
            hi.merchants += 6 * n;
            hi.peasants -= 6 * n;
        }
        InvestmentType::GrainMills => {
            lo.grain_mills += n;
            hi.grain_mills += n;
        }
        InvestmentType::Foundries => {
            lo.foundries += n;
            hi.foundries += n;
        }
        InvestmentType::Shipyards => {
            lo.shipyards += n;
            hi.shipyards += n;
        }
        InvestmentType::Soldiers => {
            lo.soldiers += n;
            hi.soldiers += n;
        }
        InvestmentType::Palaces => {
            lo.palaces += n;
            hi.palaces += n;
            lo.nobles += n;
            hi.nobles += 3 * n;
        }
    }
    (lo, hi)
}

/// A figure between two sums.
struct Bounds {
    now: Outlook,
    low: LiveSum,
    high: LiveSum,
}

impl Bounds {
    /// `low … high` (one figure when they agree), coloured by where the range
    /// sits: green when it can't be negative, red when it can't be positive,
    /// amber when it may go either way. Deltas print their sign.
    fn figure(self, signed: bool) -> ElementBuilder {
        let tone = tone(self.now);
        self.render(signed, el(El::Strong).st([tone]))
    }

    /// `low … high` as two figures, each coloured by its own sign — the
    /// register's headline, where the worst and best cases read separately.
    fn span(self) -> ElementBuilder {
        let end = |n: i32, sum: LiveSum| {
            let tone = if n > 0 {
                St::TextSuccess
            } else if n < 0 {
                St::TextError
            } else {
                St::TextDefault
            };
            let e = el(El::Strong).st([tone]).text(&delta(n));
            if sum.terms.is_empty() {
                e
            } else {
                e.live_sum_signed(sum)
            }
        };
        // One figure while the ends agree (the runtime collapses a live
        // range the same way).
        if self.now.low == self.now.high {
            let tone = tone(self.now);
            return self.render(true, el(El::Strong).st([tone]));
        }
        el(El::Span).append([
            end(self.now.low, self.low),
            el(El::Span).st([St::TextMuted]).text(" … "),
            end(self.now.high, self.high),
        ])
    }

    fn render(self, signed: bool, e: ElementBuilder) -> ElementBuilder {
        let show = |n: i32| if signed { delta(n) } else { fmt(n) };
        let text = if self.now.low == self.now.high {
            show(self.now.low)
        } else {
            format!("{} … {}", show(self.now.low), show(self.now.high))
        };
        let e = e.st([St::TabularNums]).text(&text);
        if self.low.terms.is_empty() && self.high.terms.is_empty() {
            e
        } else if signed {
            e.live_range_signed(self.low, self.high)
        } else {
            e.live_range(self.low, self.high)
        }
    }
}

fn tone(o: Outlook) -> St {
    if o.low > 0 {
        St::TextSuccess
    } else if o.high < 0 {
        St::TextError
    } else if o.low < 0 && o.high > 0 {
        St::TextWarning
    } else {
        St::TextDefault
    }
}

fn txt(s: &str) -> ElementBuilder {
    el(El::Span).text(s)
}

/// `+low … +high`, one figure when they agree.
fn outlook_text(o: Outlook) -> String {
    if o.low == o.high {
        delta(o.low)
    } else {
        format!("{} … {}", delta(o.low), delta(o.high))
    }
}

// ---------------------------------------------------------------------------
// The register: the same six cells in the roll and in every sheet, the
// cells a sheet's slider moves written "avant → après", the others dimmed.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    People,
    Nobles,
    Merchants,
    Ost,
    Treasury,
    Reserve,
}

impl Cell {
    const ALL: [Cell; 6] = [
        Cell::People,
        Cell::Nobles,
        Cell::Merchants,
        Cell::Ost,
        Cell::Treasury,
        Cell::Reserve,
    ];

    fn label(self) -> &'static str {
        match self {
            Cell::People => "Peuple",
            Cell::Nobles => "Nobles",
            Cell::Merchants => "Marchands",
            Cell::Ost => "Ost",
            Cell::Treasury => "Trésor",
            Cell::Reserve => "Réserve après",
        }
    }

    /// What a setting moves, per `demography.rs` and `economy.rs`.
    fn moved_by(along: Along) -> &'static [Cell] {
        match along {
            Along::Field(Field::Peasants) => &[
                Cell::People,
                Cell::Nobles,
                Cell::Merchants,
                Cell::Treasury,
                Cell::Reserve,
            ],
            Along::Field(Field::Soldiers) => &[Cell::Ost, Cell::Reserve],
            Along::Field(Field::Customs) | Along::Field(Field::Income) => {
                &[Cell::People, Cell::Nobles, Cell::Treasury]
            }
            Along::Field(Field::Sales) => &[Cell::People, Cell::Merchants, Cell::Treasury],
            Along::Units(InvestmentType::Marketplaces) => &[Cell::Merchants, Cell::Treasury],
            Along::Units(InvestmentType::GrainMills)
            | Along::Units(InvestmentType::Foundries)
            | Along::Units(InvestmentType::Shipyards) => &[Cell::Treasury],
            Along::Units(InvestmentType::Soldiers) => &[Cell::Ost, Cell::Treasury, Cell::Reserve],
            Along::Units(InvestmentType::Palaces) => {
                &[Cell::People, Cell::Nobles, Cell::Treasury, Cell::Reserve]
            }
        }
    }
}

/// The six cells on two rows. In a sheet (`moved` non-empty) the moved cells
/// read `avant` over the live `après`; the others keep their figure, dimmed.
fn register(b: &Board, moved: &[Cell]) -> ElementBuilder {
    let in_sheet = !moved.is_empty();
    let cells = Cell::ALL.map(|cell| {
        let is_moved = moved.contains(&cell);
        let (before, after, gauge) = cell_parts(b, cell, is_moved);
        let mut lines = vec![el(El::Div)
            .st([
                St::TextXs,
                St::TextUppercase,
                St::TrackingWider,
                St::FontSemibold,
                St::TextMuted,
                St::WhitespaceNowrap,
            ])
            .text(cell.label())];
        if in_sheet {
            lines.push(
                el(El::Div)
                    .st([
                        St::TextXs,
                        St::TextMuted,
                        St::TabularNums,
                        St::WhitespaceNowrap,
                    ])
                    .text(if is_moved { &before } else { "\u{a0}" }),
            );
        }
        let mut figure = el(El::Div)
            .st([
                St::TextSm,
                St::FontBold,
                St::LeadingTight,
                St::TrackingTight,
                St::TabularNums,
                St::WhitespaceNowrap,
                St::OverflowHidden,
            ])
            .append([after]);
        if in_sheet && !is_moved {
            figure = figure.st([St::Opacity50]);
        }
        lines.push(figure);
        lines.push(gauge.st([St::MtXs, St::MbXs]));
        el(El::Div).st([St::MinW0]).append(lines)
    });
    el(El::Div)
        .st([St::DisplayGrid, St::GridCols3, St::GapXMd, St::GapYSm])
        .append(cells)
}

/// One cell: its `avant` text, its `après` figure (live when the board has
/// an axis it depends on) and its gauge.
fn cell_parts(b: &Board, cell: Cell, live: bool) -> (String, ElementBuilder, ElementBuilder) {
    let k = b.k;
    let delta_cell = |figure: fn(&Prospect) -> Outlook| {
        let after = if live {
            b.bounds(figure).span()
        } else {
            Bounds {
                now: figure(&b.now),
                low: LiveSum {
                    base: 0,
                    terms: Vec::new(),
                },
                high: LiveSum {
                    base: 0,
                    terms: Vec::new(),
                },
            }
            .span()
        };
        (
            outlook_text(figure(&b.before)),
            after,
            range_gauge(b, figure),
        )
    };
    match cell {
        Cell::People => delta_cell(|p| p.people),
        Cell::Nobles => delta_cell(|p| p.nobles),
        Cell::Merchants => delta_cell(|p| p.merchants),
        Cell::Treasury => delta_cell(|p| p.net),
        Cell::Ost if b.find(Along::Units(InvestmentType::Soldiers)).is_some() => {
            // Recruits: the headcount against what the nobles can lead.
            let cap = (k.nobles * 20).max(1);
            let men = el(El::Span).append([
                el(El::Strong)
                    .text(&fmt(b.now.soldiers))
                    .live_sum(b.sum(|p| p.soldiers)),
                txt(" hommes"),
            ]);
            let gauge = gauge([el(El::Div)
                .st([St::GaugeSpan, St::BgWarning])
                .style(
                    Style::new()
                        .set("left", "0")
                        .width(&pct_of(b.now.soldiers, cap)),
                )
                .live_span(
                    LiveSum {
                        base: 0,
                        terms: Vec::new(),
                    },
                    b.sum(|p| p.soldiers),
                    (0, cap),
                    (0, cap),
                )]);
            (hommes(b.before.soldiers), men, gauge)
        }
        Cell::Ost => {
            let pct = el(El::Span).append([efficiency(b), txt(" %")]);
            let gauge = gauge([
                gauge_tick(100.0 / 150.0),
                el(El::Div)
                    .st([St::GaugeSpan, St::BgWarning])
                    .style(
                        Style::new()
                            .set("left", "0")
                            .width(&pct_of(b.now.efficiency, 150)),
                    )
                    .live_span(
                        LiveSum {
                            base: 0,
                            terms: Vec::new(),
                        },
                        b.sum(|p| p.efficiency),
                        (0, 150),
                        (0, 150),
                    ),
            ]);
            (format!("{} %", b.before.efficiency), pct, gauge)
        }
        Cell::Reserve => {
            let stocks = k.grain_stocks.max(1);
            let left = el(El::Span).append([
                el(El::Strong)
                    .text(&fmt(b.now.reserve))
                    .live_sum(b.sum(|p| p.reserve)),
                txt(" bx"),
            ]);
            let gauge = gauge([el(El::Div)
                .st([St::GaugeSpan, St::BgAccent])
                .style(
                    Style::new()
                        .set("left", "0")
                        .width(&pct_of(b.now.reserve.max(0), stocks)),
                )
                .live_span(
                    LiveSum {
                        base: 0,
                        terms: Vec::new(),
                    },
                    b.sum(|p| p.reserve),
                    (0, stocks),
                    (0, stocks),
                )]);
            (format!("{} bx", fmt(b.before.reserve)), left, gauge)
        }
    }
}

/// A ration as a share of the full one, in per cent, following the thumb.
fn ration_pct(b: &Board, field: Field) -> ElementBuilder {
    let full = match field {
        Field::Peasants => Council::PEASANTS_FULL,
        _ => Council::SOLDIERS_FULL,
    };
    el(El::Span)
        .text(&(b.draft.get(field) * 100 / full).to_string())
        .live_scaled(b.slider().channel, 100, full as u32)
}

/// A gauge strip: the axis, then `marks` (ticks and spans) over it.
fn gauge(marks: impl IntoIterator<Item = ElementBuilder>) -> ElementBuilder {
    el(El::Div)
        .st([St::Gauge])
        .append([el(El::Div).st([St::GaugeAxis])])
        .append(marks)
}

/// A tick at `at` (0 … 1) along a gauge.
fn gauge_tick(at: f64) -> ElementBuilder {
    el(El::Div)
        .st([St::GaugeTick])
        .style(Style::new().set("left", &format!("{:.1}%", at * 100.0)))
}

/// `n / of` as a CSS percentage.
fn pct_of(n: i32, of: i32) -> String {
    format!(
        "{:.1}%",
        if of > 0 {
            n as f64 / of as f64 * 100.0
        } else {
            0.0
        }
    )
}

/// A delta's gauge: the zero tick on an axis wide enough for the figure along
/// every slider, a red span below zero and a green one above, both following
/// the thumbs.
fn range_gauge(b: &Board, figure: fn(&Prospect) -> Outlook) -> ElementBuilder {
    let (mut lo, mut hi) = b
        .axes
        .iter()
        .flat_map(|a| a.curve.iter())
        .chain([&b.before, &b.now])
        .map(figure)
        .fold((0, 0), |(lo, hi), o| (lo.min(o.low), hi.max(o.high)));
    let pad = ((hi - lo) / 20).max(1);
    lo -= pad;
    hi += pad;
    let at = |v: i32| (v - lo) as f64 / (hi - lo) as f64;
    let now = b.now;
    let span = |tone: St, window: (i32, i32)| {
        let (from, to) = (
            figure(&now).low.clamp(window.0, window.1),
            figure(&now).high.clamp(window.0, window.1),
        );
        let bounds = b.bounds(figure);
        el(El::Div)
            .st([St::GaugeSpan, tone])
            .style(
                Style::new()
                    .set("left", &format!("{:.1}%", at(from) * 100.0))
                    .width(&format!("{:.1}%", (at(to) - at(from)) * 100.0)),
            )
            .live_span(bounds.low, bounds.high, (lo, hi), window)
    };
    gauge([
        gauge_tick(at(0)),
        span(St::BgError, (lo, 0)),
        span(St::BgSuccess, (0, hi)),
    ])
}

/// The army's efficiency, red under full rations; follows the ration slider
/// when the board has one.
fn efficiency(b: &Board) -> ElementBuilder {
    let full = Council::SOLDIERS_FULL;
    let now = b.now.efficiency;
    let Some(axis) = b.find(Along::Field(Field::Soldiers)) else {
        let tone = if now < 100 {
            St::TextError
        } else {
            St::TextSuccess
        };
        return el(El::Strong)
            .st([tone, St::TabularNums])
            .text(&now.to_string());
    };
    let pct: Vec<i32> = axis.curve.iter().map(|p| p.efficiency).collect();
    let figure = |tone: St, shown: bool| {
        hidden_unless(
            shown,
            el(El::Strong)
                .st([tone, St::TabularNums])
                .text(&now.to_string())
                .live_lookup(axis.channel, &pct),
        )
    };
    el(El::Span)
        .live_switch(axis.channel, &[full as u32])
        .append([
            figure(St::TextError, b.draft.soldiers < full),
            figure(St::TextSuccess, b.draft.soldiers >= full),
        ])
}

// ---------------------------------------------------------------------------
// Conseil: one sheet per setting — the register, the sentence, the curve
// over the slider, the verb — settled into the draft by `room::settle`.
// ---------------------------------------------------------------------------

fn council_sheet(t: T, id: Kingdoms, field: Field) -> ElementBuilder {
    let k = t.room.game.kingdom(id);
    let b = Board::along(k, t.room.game.weather, t.room.draft(id), field);
    let council = b.draft.council();
    let (sub, verb) = match field {
        Field::Peasants => (
            format!(
                "{} habitants · {} bx",
                fmt(k.population()),
                fmt(council.grain_for_peasants(k))
            ),
            "Fixer la ration",
        ),
        Field::Soldiers => (
            format!(
                "{} hommes · {} bx",
                fmt(k.soldiers),
                fmt(council.grain_for_soldiers(k))
            ),
            "Fixer la ration",
        ),
        Field::Customs => (
            "sur chaque étranger qui entre".to_string(),
            "Fixer la douane",
        ),
        Field::Sales => (
            "commerce du royaume, nobles, serfs".to_string(),
            "Fixer la gabelle",
        ),
        Field::Income => ("paysans, nobles, domaines".to_string(), "Fixer la taille"),
    };
    let (sentence, slider) = match field {
        Field::Peasants => people_slider(&b),
        Field::Soldiers => ost_slider(&b),
        _ => tax_slider(&b, field),
    };
    form(
        by(room::settle(), t.token, t.code, &[field as u8]),
        [
            subtitle(vec![txt(&sub)]),
            register(&b, Cell::moved_by(Along::Field(field))),
            sentence,
            slider,
            Button::primary(verb)
                .size(ButtonSize::Lg)
                .full_width(true)
                .build(),
        ],
    )
}

/// Peuple: the sentence per band and the ration over the headcount curve.
fn people_slider(b: &Board) -> (ElementBuilder, ElementBuilder) {
    let k = b.k;
    let full = Council::PEASANTS_FULL;
    let axis = b.slider();
    let value = b.draft.peasants;
    let curve: Vec<Outlook> = axis.curve.iter().map(|p| p.people).collect();
    (
        consequences(
            axis.channel,
            value,
            &[full / 2, full, full * 5 / 4],
            &[
                "Famine : des sujets meurent de faim, les naissances s'effondrent, la cour se disperse.",
                "Mal nourri : la faim fait des victimes, les naissances baissent.",
                "Nourri : le peuple vit ; passé la ration pleine, les premiers étrangers se présentent à la douane.",
                "Bien nourri : les étrangers immigrent, des nobles s'installent, la douane rapporte.",
            ],
        ),
        ration_slider(
            b,
            Field::Peasants,
            "bx par tête ·",
            k.mouths(),
            &[
                (full, "5 bx · 100 %"),
                (full * 3 / 2, "7,5 · 150 %"),
                (full * 2, "10 · 200 %"),
            ],
            people_chart(axis.channel, value, axis.max, &curve),
        ),
    )
}

/// Ost: the sentence per band and the ration over the efficiency curve.
/// With no men the rate still matters: it is what the recruits will eat,
/// and fight at.
fn ost_slider(b: &Board) -> (ElementBuilder, ElementBuilder) {
    let k = b.k;
    let full = Council::SOLDIERS_FULL;
    let axis = b.slider();
    let value = b.draft.soldiers;
    let curve: Vec<(i32, Outlook)> = axis
        .curve
        .iter()
        .map(|p| (p.efficiency, p.losses))
        .collect();
    let sentence = if k.soldiers > 0 {
        consequences(
            axis.channel,
            value,
            &[full / 2, full, full * 3 / 2],
            &[
                "Affamé, l'ost perd des hommes et déserte.",
                "Rations réduites : des hommes désertent, la force chute vite.",
                "L'ost combattra à pleine force ; mieux encore à 150 %.",
                "L'ost combattra à 150 %, son maximum.",
            ],
        )
    } else {
        el(El::P).st([St::TextSm, St::TextMuted, St::M0]).append([
            txt("Personne à nourrir cette année. Chaque recrue mangera "),
            el(El::Span)
                .st([St::TabularNums])
                .text(&tenths(value))
                .live_decimal(axis.channel, 1),
            txt(" bx et se battra à "),
            efficiency(b),
            txt(" %."),
        ])
    };
    (
        sentence,
        ration_slider(
            b,
            Field::Soldiers,
            "bx par homme ·",
            k.soldiers,
            &[(full, "8 bx · 100 %"), (full * 3 / 2, "12 · 150 %")],
            ost_chart(axis.channel, k, value, axis.max, &curve),
        ),
    )
}

/// What a tax says, per tax.
struct TaxCopy {
    label: &'static str,
    revenue: fn(&Prospect) -> Outlook,
    marks: &'static [(i32, &'static str)],
    thresholds: [i32; 3],
    sentences: [&'static str; 4],
}

const CUSTOMS: TaxCopy = TaxCopy {
    label: "Droits de douane",
    revenue: |p| p.customs,
    marks: &[(25, "25 %"), (50, "50 %")],
    thresholds: [10, 30, 45],
    sentences: [
        "Frontières ouvertes : presque tous entrent, la douane rapporte peu.",
        "Péage raisonnable : la recette monte plus vite que les entrées ne baissent.",
        "Au-delà du pic : chaque point coûte plus d'immigrants qu'il ne rapporte.",
        "Frontière fermée : plus personne n'entre, plus rien à percevoir.",
    ],
};

const SALES: TaxCopy = TaxCopy {
    label: "Gabelle",
    revenue: |p| p.sales,
    marks: &[(10, "10 %"), (20, "20 %")],
    thresholds: [5, 12, 17],
    sentences: [
        "Marchands choyés : les boutiques s'ouvrent, la gabelle rapporte peu.",
        "Gabelle modérée : le commerce ralentit un peu, la recette suit.",
        "Le commerce s'étiole : des marchands ferment boutique.",
        "Les marchands fuient la gabelle, les foires se vident.",
    ],
};

const INCOME: TaxCopy = TaxCopy {
    label: "Taille",
    revenue: |p| p.income,
    marks: &[(20, "20 %"), (35, "35 %")],
    thresholds: [12, 24, 30],
    sentences: [
        "Taille légère : le peuple prospère, le trésor moins.",
        "Taille ordinaire : la cour reste, les naissances fléchissent un peu.",
        "Taille lourde : les nobles quittent la cour, les berceaux se vident.",
        "Taille écrasante : les berceaux se vident, la cour aussi, les moulins tournent au ralenti.",
    ],
};

fn tax_copy(field: Field) -> TaxCopy {
    match field {
        Field::Customs => CUSTOMS,
        Field::Sales => SALES,
        Field::Income => INCOME,
        Field::Peasants | Field::Soldiers => unreachable!("rations have their own sliders"),
    }
}

/// A tax: the sentence per band and the rate over the revenue curve, the
/// revenue in the readout.
fn tax_slider(b: &Board, field: Field) -> (ElementBuilder, ElementBuilder) {
    let cur = b.k.currency();
    let axis = b.slider();
    let ch = axis.channel;
    let value = b.draft.get(field);
    let copy = tax_copy(field);
    let curve: Vec<Outlook> = axis.curve.iter().map(copy.revenue).collect();
    let revenue = el(El::Span)
        .st([St::TextSm, St::TextMuted, St::TabularNums])
        .append([
            txt(" · "),
            b.bounds(copy.revenue).figure(false),
            txt(" "),
            live_coin_word(ch, cur, value, 0, axis.max, &|v| curve[v as usize].high),
        ]);
    let mut slider = Slider::new()
        .name(field.name())
        .id(field.name())
        .channel(ch)
        .label("Taux")
        .unit("%")
        .min(0)
        .max(axis.max)
        .value(value)
        .readout_suffix(revenue)
        .above_track(revenue_chart(ch, value, axis.max, &curve));
    for &(at, label) in copy.marks {
        slider = slider.mark(at, label);
    }
    (
        consequences(ch, value, &copy.thresholds, &copy.sentences),
        slider.build(),
    )
}

/// A ration slider in tenths of a bushel per head, reading
/// `5,0 bx par tête · 100 % · 10 140 bx` — the rate, its share of the full
/// ration and what it comes to for `heads` — with the chart between readout
/// and track.
fn ration_slider(
    b: &Board,
    field: Field,
    unit: &'static str,
    heads: i32,
    marks: &[(i32, &'static str)],
    chart: ElementBuilder,
) -> ElementBuilder {
    let axis = b.slider();
    let value = b.draft.get(field);
    let mut s = Slider::new()
        .name(field.name())
        .id(field.name())
        .channel(axis.channel)
        .decimals(1, tenths)
        .label("Ration")
        .unit(unit)
        .min(0)
        .max(axis.max)
        .value(value)
        .readout_suffix(
            el(El::Span)
                .st([St::TextSm, St::TextMuted, St::TabularNums])
                .append([
                    ration_pct(b, field).st([St::TextDefault, St::FontSemibold]),
                    txt(" % · "),
                    el(El::Span)
                        .text(&fmt(value * heads / RATION_SCALE))
                        .live_scaled_grouped(axis.channel, heads as u32, RATION_SCALE as u32),
                    txt(" bx"),
                ]),
        )
        .above_track(chart);
    for &(at, label) in marks {
        s = s.mark(at, label);
    }
    s.build()
}

// The outcome curves under the ration sliders: the lib's expected figures
// at every point of the slider's run, drawn as an SVG and read back
// client-side by `live_lookup` as the thumb moves.

const CHART_W: f64 = 320.0;
const CHART_H: f64 = 96.0;
const CHART_PAD: f64 = 8.0;

/// The x of point `i` of `n` spread over the chart's width.
fn x_at(i: usize, n: usize) -> f64 {
    i as f64 / (n - 1).max(1) as f64 * CHART_W
}

fn curve_path(points: impl Iterator<Item = (f64, f64)>) -> String {
    points
        .enumerate()
        .map(|(i, (x, y))| format!("{}{x:.1} {y:.1}", if i == 0 { 'M' } else { 'L' }))
        .collect()
}

/// A closed area: along `top`, back along `bottom`.
fn area_path(top: &[(f64, f64)], bottom: &[(f64, f64)]) -> String {
    let mut d = curve_path(top.iter().copied());
    for (x, y) in bottom.iter().rev() {
        d.push_str(&format!("L{x:.1} {y:.1}"));
    }
    d.push('Z');
    d
}

fn stroke(d: &str, color: &str, width: &str) -> ElementBuilder {
    el(El::Path)
        .at_str(At::D, d)
        .at(At::Fill, Av::None)
        .at_str(At::Stroke, color)
        .at_str(At::StrokeWidth, width)
        .at(At::StrokeLinejoin, Av::Round)
}

fn fill(d: &str, color: &str, opacity: &str) -> ElementBuilder {
    el(El::Path)
        .at_str(At::D, d)
        .at_str(At::Fill, color)
        .attr("opacity", opacity)
}

fn gridline(y: f64, dashed: bool) -> ElementBuilder {
    let line = stroke(
        &format!("M0 {y:.1}H{CHART_W}"),
        "var(--h)",
        if dashed { "0.7" } else { "1" },
    );
    if dashed {
        line.attr("stroke-dasharray", "2 3")
    } else {
        line
    }
}

/// The SVG in a relative box, with axis labels at the right and a cursor whose
/// right edge follows the slider.
fn chart_frame(
    channel: u16,
    value: i32,
    max: i32,
    paths: Vec<ElementBuilder>,
    labels: Vec<(f64, String)>,
) -> ElementBuilder {
    let pct = if max > 0 {
        value as f64 / max as f64 * 100.0
    } else {
        0.0
    };
    el(El::Div)
        .st([St::PositionRelative, St::WFull])
        .append([el(El::Svg)
            .at_str(At::ViewBox, &format!("0 0 {CHART_W} {CHART_H}"))
            .at_str(At::Width, "100%")
            .st([St::DisplayBlock])
            .append(paths)])
        .append(labels.into_iter().map(|(y, text)| {
            el(El::Span)
                .st([St::ChartLabel, St::TabularNums])
                .style(Style::new().set("top", &format!("{:.1}%", y / CHART_H * 100.0)))
                .text(&text)
        }))
        .append([el(El::Div)
            .st([St::ChartCursor])
            .style(Style::new().width(&format!("{pct:.1}%")))
            .live_fill(channel)])
}

/// Net headcount over the year: the expected curve with its spread, green
/// above zero and red below, on an asinh scale (linear around zero, compressed
/// toward the famine).
fn people_chart(channel: u16, value: i32, max: i32, curve: &[Outlook]) -> ElementBuilder {
    let top = curve.iter().map(|o| o.high).max().unwrap_or(0).max(1) as f64;
    let bottom = curve.iter().map(|o| o.low).min().unwrap_or(0).min(-1) as f64;
    let scale = (top.max(-bottom) / 16.0).max(1.0);
    let (hi, lo) = ((top * 1.1 / scale).asinh(), (bottom * 1.05 / scale).asinh());
    let y =
        |v: f64| CHART_PAD + (hi - (v / scale).asinh()) / (hi - lo) * (CHART_H - 2.0 * CHART_PAD);
    let y0 = y(0.0);

    let pts = |f: &dyn Fn(&Outlook) -> f64| -> Vec<(f64, f64)> {
        curve
            .iter()
            .enumerate()
            .map(|(i, o)| (x_at(i, curve.len()), y(f(o))))
            .collect()
    };
    let expected = pts(&|o| o.expected as f64);
    let zero: Vec<(f64, f64)> = expected.iter().map(|&(x, _)| (x, y0)).collect();
    let above: Vec<(f64, f64)> = expected.iter().map(|&(x, y)| (x, y.min(y0))).collect();
    let below: Vec<(f64, f64)> = expected.iter().map(|&(x, y)| (x, y.max(y0))).collect();

    // Axis ticks: the largest round figure below the top and the bottom.
    let round_down = |v: f64| {
        [
            10_000.0, 5_000.0, 2_000.0, 1_000.0, 500.0, 200.0, 100.0, 50.0, 20.0,
        ]
        .into_iter()
        .find(|&t| t <= v * 0.9)
    };
    let mut labels = vec![(y0, "0".to_string())];
    let mut paths = vec![gridline(y0, false)];
    if let Some(t) = round_down(top) {
        paths.push(gridline(y(t), true));
        labels.push((y(t), format!("+{}", fmt(t as i32))));
    }
    if let Some(t) = round_down(-bottom) {
        paths.push(gridline(y(-t), true));
        labels.push((y(-t), format!("−{}", fmt(t as i32))));
    }
    paths.extend([
        fill(&area_path(&above, &zero), "var(--o)", ".18"),
        fill(&area_path(&below, &zero), "var(--q)", ".18"),
        fill(
            &area_path(&pts(&|o| o.high as f64), &pts(&|o| o.low as f64)),
            "var(--k)",
            ".12",
        ),
        stroke(&curve_path(expected.into_iter()), "var(--k)", "1.8"),
    ]);
    chart_frame(channel, value, max, paths, labels)
}

/// The army's efficiency line and its losses band, both in % of the men.
fn ost_chart(
    channel: u16,
    k: &Kingdom,
    value: i32,
    max: i32,
    curve: &[(i32, Outlook)],
) -> ElementBuilder {
    let y = |pct: f64| {
        CHART_PAD + (150.0 - pct.clamp(0.0, 150.0)) / 150.0 * (CHART_H - 2.0 * CHART_PAD)
    };
    let mut paths = vec![
        gridline(y(150.0), true),
        gridline(y(100.0), false),
        gridline(y(50.0), true),
    ];
    let labels = vec![
        (y(150.0), "150 %".to_string()),
        (y(100.0), "100 %".to_string()),
        (y(50.0), "50 %".to_string()),
    ];
    if k.soldiers > 0 {
        // The smooth curve behind the rounded figures, so a small army doesn't
        // draw a staircase.
        let (share, spread): (Vec<f64>, Vec<f64>) = (0..=max)
            .map(|rate| {
                let (s, spread) = army_losses_share(rate);
                (s as f64 * 100.0, spread as f64)
            })
            .unzip();
        let pts = |scale: f64| -> Vec<(f64, f64)> {
            share
                .iter()
                .enumerate()
                .map(|(i, s)| (x_at(i, share.len()), y(s * scale)))
                .collect()
        };
        let spread = spread.first().copied().unwrap_or(0.0);
        paths.extend([
            fill(
                &area_path(&pts(1.0 + spread), &pts(1.0 - spread)),
                "var(--q)",
                ".2",
            ),
            stroke(&curve_path(pts(1.0).into_iter()), "var(--q)", "1.2"),
        ]);
    }
    paths.push(stroke(
        &curve_path(
            curve
                .iter()
                .enumerate()
                .map(|(i, (e, _))| (x_at(i, curve.len()), y(*e as f64))),
        ),
        "var(--p)",
        "1.8",
    ));
    chart_frame(channel, value, max, paths, labels)
}

/// Tenths as a decimal, `5,0`, as `live_decimal` prints it in French.
fn tenths(n: i32) -> String {
    format!("{},{}", fmt(n / RATION_SCALE), n % RATION_SCALE)
}

/// "+1 234" / "−56" / "0", as `live_lookup_signed` prints it.
fn delta(n: i32) -> String {
    let sign = if n > 0 {
        "+"
    } else if n < 0 {
        "−"
    } else {
        ""
    };
    format!("{sign}{}", fmt(n.abs()))
}

/// A tax's revenue band, from nothing to the best it can bring in.
fn revenue_chart(channel: u16, value: i32, max: i32, curve: &[Outlook]) -> ElementBuilder {
    let top = curve.iter().map(|o| o.high).max().unwrap_or(0).max(1) as f64;
    let y =
        |v: i32| CHART_PAD + (top - v as f64).clamp(0.0, top) / top * (CHART_H - 2.0 * CHART_PAD);
    let x = |i: usize| {
        CHART_PAD + i as f64 / (curve.len() - 1).max(1) as f64 * (CHART_W - 2.0 * CHART_PAD)
    };
    let pts = |pick: fn(&Outlook) -> i32| -> Vec<(f64, f64)> {
        curve
            .iter()
            .enumerate()
            .map(|(i, o)| (x(i), y(pick(o))))
            .collect()
    };
    let half = (top / 2.0).round() as i32;
    let paths = vec![
        gridline(y(top as i32), true),
        gridline(y(half), true),
        gridline(y(0), false),
        fill(
            &area_path(&pts(|o| o.high), &pts(|o| o.low)),
            "var(--k)",
            ".12",
        ),
        stroke(
            &curve_path(pts(|o| o.expected).into_iter()),
            "var(--k)",
            "1.8",
        ),
    ];
    let labels = vec![
        (y(top as i32), fmt(top as i32)),
        (y(half), fmt(half)),
        (y(0), "0".to_string()),
    ];
    chart_frame(channel, value, max, paths, labels)
}

/// The sentence under a slider: one per band, thresholds ascending; the client
/// swaps them as the thumb moves.
fn consequences(
    channel: u16,
    value: i32,
    thresholds: &[i32],
    sentences: &[&'static str],
) -> ElementBuilder {
    let band = thresholds.iter().filter(|&&t| value >= t).count();
    let limits: Vec<u32> = thresholds.iter().map(|&t| t.max(0) as u32).collect();
    el(El::P)
        .st([St::TextSm, St::TextMuted, St::MtXs])
        .live_switch(channel, &limits)
        .append(
            sentences
                .iter()
                .enumerate()
                .map(|(i, s)| hidden_unless(i == band, el(El::Span).text(s))),
        )
}

fn hidden_unless(shown: bool, e: ElementBuilder) -> ElementBuilder {
    if shown {
        e
    } else {
        e.bool_attr(At::Hidden)
    }
}

/// The currency word after a figure: "franc" for one (or none), "francs" otherwise.
fn coin_word(n: i32, cur: &str) -> &str {
    if n.abs() <= 1 {
        cur.strip_suffix('s').unwrap_or(cur)
    } else {
        cur
    }
}

/// "1 234 francs" / "1 franc".
pub fn coins(n: i32, cur: &str) -> String {
    format!("{} {}", fmt(n), coin_word(n, cur))
}

/// The currency word after a live figure `f(v)` of slider `ch` (`f` monotone
/// over `min..=max`): singular on the run of values where the figure is one
/// or none, plural on either side of it.
fn live_coin_word(
    ch: u16,
    cur: &str,
    value: i32,
    min: i32,
    max: i32,
    f: &dyn Fn(i32) -> i32,
) -> ElementBuilder {
    let mut single = (min..=max).filter(|&v| f(v).abs() <= 1);
    let from = single.next();
    let upto = single.last().or(from);
    let (from, upto) = match (from, upto) {
        (Some(a), Some(b)) => (a, b + 1),
        _ => (max + 1, max + 1),
    };
    let band = [from, upto].iter().filter(|&&t| value >= t).count();
    let words = [cur, coin_word(1, cur), cur];
    el(El::Span)
        .live_switch(ch, &[from.max(0) as u32, upto.max(0) as u32])
        .append(
            words
                .iter()
                .enumerate()
                .map(|(i, w)| hidden_unless(i == band, el(El::Span).text(w))),
        )
}

/// Peuple: the census counts from last year's headcount to this year's while
/// the causes appear one by one, each with a bar.
fn report_step(seat: &Seat) -> ElementBuilder {
    let Some(d) = &seat.demo else {
        return el(El::Div);
    };
    let change = d.population_delta();
    let after = seat.population_after;
    let before = after - change;

    let census = el(El::Div)
        .st([St::TextCenter, St::PtMd, St::PbSm])
        .append([
            reveal(
                0,
                el(El::Div)
                    .st([
                        St::TextXs,
                        St::TextMuted,
                        St::TextUppercase,
                        St::TrackingWider,
                    ])
                    .text("Habitants"),
            ),
            el(El::Div)
                .st([St::Text4xl, St::FontBold, St::LeadingNone, St::MtXs])
                .append([count_up(before, after, 0)]),
            reveal(
                COUNT_SLOTS,
                el(El::Div).st([St::MtSm]).append([match change {
                    n if n > 0 => Badge::success(format!("+{} sujets", fmt(n))),
                    n if n < 0 => Badge::error(format!("−{} sujets", fmt(-n))),
                    _ => Badge::default_badge("population stable"),
                }
                .build()]),
            ),
        ]);

    let causes = [
        (d.births, "Naissances"),
        (
            d.immigrants - d.nobles_immigrants - d.merchants_immigrants,
            "Étrangers venus s'installer",
        ),
        (d.nobles_immigrants, "Nobles venus à la cour"),
        (d.merchants_immigrants, "Marchands venus d'ailleurs"),
        (d.merchants_settled, "Marchands nouvellement établis"),
        (-d.disease_victims, "Morts de maladie"),
        (-d.malnutrition_victims, "Morts de faim"),
        (-d.starvation_victims, "Morts de misère"),
        (-d.nobles_departed, "Nobles ayant quitté la cour"),
        (-d.merchants_departed, "Marchands ayant fermé boutique"),
        (
            -d.soldiers_starvation_victims,
            "Hommes d'armes morts d'épuisement",
        ),
        (-d.soldiers_desertion_victims, "Hommes d'armes déserteurs"),
    ];
    let largest = causes
        .iter()
        .map(|(n, _)| n.abs())
        .max()
        .unwrap_or(0)
        .max(1);
    let mut slot = 1;
    let mut rows: Vec<ElementBuilder> = causes
        .iter()
        .filter(|(n, _)| *n != 0)
        .map(|(n, label)| {
            let intent = if *n > 0 {
                ProgressIntent::Success
            } else {
                ProgressIntent::Error
            };
            let bar = Progress::new()
                .value(n.unsigned_abs())
                .max(largest as u32)
                .intent(intent)
                .thin(true)
                .bar_st([St::AnimateGrow, delay(slot)])
                .build();
            let row = ledger_row(slot, el(El::Span).text(label), signed(*n), Some(bar));
            slot += 1;
            row
        })
        .collect();
    if rows.is_empty() {
        rows.push(reveal(
            slot,
            Text::body("Une année sans histoire.").muted().build(),
        ));
        slot += 1;
    }

    let efficiency = d.soldiers_efficiency;
    rows.push(ledger_row(
        slot,
        el(El::Span).text("L'ost combattra à"),
        el(El::Span).text(&format!("{efficiency} %")),
        Some(
            Progress::new()
                .value(efficiency.max(0) as u32)
                .max(200)
                .intent(ProgressIntent::Warning)
                .thin(true)
                .bar_st([St::AnimateGrow, delay(slot)])
                .build(),
        ),
    ));
    slot += 1;
    rows.push(reveal(
        slot,
        el(El::P)
            .st([
                St::Italic,
                St::TextMuted,
                St::TextSm,
                St::TextCenter,
                St::PtMd,
            ])
            .text(&format!(
                "Vous avez {} {} sujets taillables et corvéables à merci.",
                room::delta_verb(change),
                fmt(change.abs())
            )),
    ));

    Stack::column()
        .gap(Gap::Md)
        .children([census, el(El::Div).st([St::PxXs]).append(rows)])
        .build()
}

/// Trésor: the same mould as the census — the treasury counts from last
/// year's balance to this year's, then every source of income appears in
/// turn, the largest first, the cost of the ost last.
fn treasury_step(k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let Some(e) = &seat.eco else {
        return el(El::Div);
    };
    let change = e.net();
    let after = seat.treasury_after;
    let before = after - change;
    let cur = k.currency();

    let kpi = el(El::Div)
        .st([St::TextCenter, St::PtMd, St::PbSm])
        .append([
            reveal(
                0,
                el(El::Div)
                    .st([
                        St::TextXs,
                        St::TextMuted,
                        St::TextUppercase,
                        St::TrackingWider,
                    ])
                    .text("Trésor"),
            ),
            el(El::Div)
                .st([St::Text4xl, St::FontBold, St::LeadingNone, St::MtXs])
                .append([count_up(before, after, 0)]),
            reveal(
                COUNT_SLOTS,
                el(El::Div).st([St::MtSm]).append([match change {
                    n if n > 0 => Badge::success(format!("+{}", coins(n, cur))),
                    n if n < 0 => Badge::error(format!("−{}", coins(-n, cur))),
                    _ => Badge::default_badge("trésor inchangé"),
                }
                .build()]),
            ),
        ]);

    let rate = |r: i32| format!("{r} %");
    let mut sources = vec![
        (
            "Champs de foire",
            fmt(k.marketplaces),
            e.marketplaces_profits,
        ),
        ("Moulins à grain", fmt(k.grain_mills), e.grain_mills_profits),
        (
            "Commerce des armes",
            format!("{} fonderies", fmt(k.foundries)),
            e.foundries_profits,
        ),
        ("Chantiers navals", fmt(k.shipyards), e.shipyards_profits),
        (
            "Droits de douane",
            rate(k.immigration_taxes),
            e.immigration_taxes_profits,
        ),
        (
            "Gabelle",
            rate(k.commercial_taxes),
            e.commercial_taxes_profits,
        ),
        ("Taille", rate(k.income_taxes), e.income_taxes_profits),
        (
            "Solde de l'ost",
            hommes(k.soldiers),
            -e.soldiers_maintenance,
        ),
    ];
    sources.retain(|(_, _, n)| *n != 0);
    sources.sort_by_key(|(_, _, n)| (*n < 0, -n.abs()));
    let largest = sources
        .iter()
        .map(|(_, _, n)| n.abs())
        .max()
        .unwrap_or(0)
        .max(1);
    let mut slot = 1;
    let mut rows: Vec<ElementBuilder> = sources
        .iter()
        .map(|(label, note, n)| {
            let intent = if *n > 0 {
                ProgressIntent::Success
            } else {
                ProgressIntent::Error
            };
            let bar = Progress::new()
                .value(n.unsigned_abs())
                .max(largest as u32)
                .intent(intent)
                .thin(true)
                .bar_st([St::AnimateGrow, delay(slot)])
                .build();
            let row = ledger_row(
                slot,
                aside(label, &format!("· {note}")),
                signed(*n),
                Some(bar),
            );
            slot += 1;
            row
        })
        .collect();
    if rows.is_empty() {
        rows.push(reveal(
            slot,
            Text::body("Pas un sou n'est entré ni sorti.")
                .muted()
                .build(),
        ));
        slot += 1;
    }
    rows.push(reveal(
        slot,
        el(El::P)
            .st([
                St::Italic,
                St::TextMuted,
                St::TextSm,
                St::TextCenter,
                St::PtMd,
            ])
            .text(&match change {
                n if n > 0 => format!("Le trésor s'est enrichi de {} cette année.", coins(n, cur)),
                n if n < 0 => format!(
                    "Le trésor s'est appauvri de {} cette année.",
                    coins(-n, cur)
                ),
                _ => "Le trésor n'a pas bougé cette année.".to_string(),
            }),
    ));

    Stack::column()
        .gap(Gap::Md)
        .children([kpi, el(El::Div).st([St::PxXs]).append(rows)])
        .build()
}

// ---------------------------------------------------------------------------
// Achats: one sheet per thing money buys — what it does, what one more
// would bring, its repercussions, the register, the quantity, the verb —
// concluded by `room::invest`. An unaffordable sheet opens all the same:
// it explains the thing and says what is missing.
// ---------------------------------------------------------------------------

/// The icon of each purchase.
fn invest_icon(kind: InvestmentType) -> Icon {
    match kind {
        InvestmentType::Marketplaces => Icon::Tent,
        InvestmentType::GrainMills => Icon::Mill,
        InvestmentType::Foundries => Icon::Anvil,
        InvestmentType::Shipyards => Icon::Ship,
        InvestmentType::Soldiers => Icon::Helmet,
        InvestmentType::Palaces => Icon::Castle,
    }
}

/// The verb of each purchase with its unit, singular and plural.
fn invest_verb(kind: InvestmentType) -> (&'static str, &'static str, &'static str) {
    match kind {
        InvestmentType::Marketplaces => ("Acheter", "champ de foire", "champs de foire"),
        InvestmentType::GrainMills => ("Acheter", "moulin", "moulins"),
        InvestmentType::Foundries => ("Acheter", "fonderie", "fonderies"),
        InvestmentType::Shipyards => ("Acheter", "chantier", "chantiers"),
        InvestmentType::Soldiers => ("Recruter", "homme", "hommes"),
        InvestmentType::Palaces => ("Bâtir", "dixième", "dixièmes"),
    }
}

/// How many the kingdom owns.
fn owned(k: &Kingdom, kind: InvestmentType) -> i32 {
    match kind {
        InvestmentType::Marketplaces => k.marketplaces,
        InvestmentType::GrainMills => k.grain_mills,
        InvestmentType::Foundries => k.foundries,
        InvestmentType::Shipyards => k.shipyards,
        InvestmentType::Soldiers => k.soldiers,
        InvestmentType::Palaces => k.palaces,
    }
}

/// What the thing does, in the rules' own terms.
fn invest_intro(kind: InvestmentType, cur: &str) -> String {
    match kind {
        InvestmentType::Marketplaces => format!(
            "Un champ de foire attire de un à six marchands, pris parmi vos serfs. Les foires \
             rapportent avec le nombre de marchands, et d'autant moins que la gabelle est lourde \
             — elle prend en retour sa part de leur commerce ; le champ vaut 99 {cur} de richesse \
             à la taille."
        ),
        InvestmentType::GrainMills => format!(
            "Le moulin vend sa farine : il rapporte avec la récolte — un bon été, un bon moulin —, \
             moins sous une taille et une gabelle lourdes. Sa mouture entre dans l'assiette de la \
             gabelle, et il vaut 99 {cur} de richesse à la taille. Il ne protège pas des rats."
        ),
        InvestmentType::Foundries => format!(
            "La fonderie forge les armes du royaume : son commerce va au trésor sans passer par \
             aucune taxe mais pèse lourd dans l'assiette de la gabelle ; elle fournit les chantiers \
             navals (+15 chacun) et vaut 425 {cur} de richesse à la taille. Un roi doit en tenir \
             une. Ses ouvriers ne labourent pas : −500 bx sur chaque récolte."
        ),
        InvestmentType::Shipyards => format!(
            "Le chantier arme des navires pour le commerce du royaume : il rapporte selon ce qu'il \
             y a à embarquer — 4 par marchand, 9 par champ de foire, 15 par fonderie — et selon le \
             temps qu'il fait. Sa recette n'est pas taxée mais pèse le plus lourd dans l'assiette \
             de la gabelle ; il vaut 965 {cur} de richesse à la taille."
        ),
        InvestmentType::Soldiers => format!(
            "L'ost défend vos terres et mène les expéditions — une par an, plus une tous les \
             quatre nobles, chaque noble menant vingt hommes. Une recrue coûte 8 {cur}, puis \
             8 {cur} de solde chaque année, et mange la ration de l'ost que fixe le conseil. Sa \
             force au combat est celle de cette ration."
        ),
        InvestmentType::Palaces => format!(
            "Chaque dixième de palais bâti attire un à trois nobles à la cour. Un noble mène vingt \
             hommes d'armes, vaut 145 {cur} de richesse à la taille, et tous les quatre nobles \
             l'ost peut mener une expédition de plus par an. La cour se disperse sous une taille \
             lourde ou quand le peuple a faim."
        ),
    }
}

/// The repercussions, as chips.
fn invest_chips(kind: InvestmentType) -> &'static [&'static str] {
    match kind {
        InvestmentType::Marketplaces => &[
            "gabelle ↑",
            "marchands +1…6",
            "serfs −1…6",
            "brûle en cas d'invasion",
        ],
        InvestmentType::GrainMills => &[
            "profit ↑ avec la récolte",
            "gabelle ↑",
            "brûle en cas d'invasion",
        ],
        InvestmentType::Foundries => &[
            "titre de roi",
            "gabelle ↑",
            "taille ↑",
            "récolte −500 bx/an",
            "brûle en cas d'invasion",
        ],
        InvestmentType::Shipyards => &[
            "profit ↑ avec marchands et foires",
            "gabelle ↑",
            "taille ↑",
            "dépend du temps",
            "brûle en cas d'invasion",
        ],
        InvestmentType::Soldiers => &[
            "défense ↑",
            "expéditions plus fortes",
            "solde −8/an chacun",
            "nourri par le conseil",
        ],
        InvestmentType::Palaces => &[
            "nobles +1…3",
            "ost jusqu'à +60",
            "expéditions ↑",
            "la cour part si taille > ~24 %",
        ],
    }
}

/// What one more would bring, from the year's outlook with and without it.
fn invest_yield(b: &Board, kind: InvestmentType, rate: i32) -> String {
    let k = b.k;
    let cur = k.currency();
    let cost = kind.cost();
    let gain = b.now.net.expected - b.before.net.expected;
    let amortized = || {
        if gain > 0 {
            format!(" · amorti en ~{} ans", (cost + gain - 1) / gain)
        } else {
            String::new()
        }
    };
    match kind {
        InvestmentType::Marketplaces => format!(
            "≈ {} {} par an au trésor, un peu plus si les marchands affluent{}. Les chantiers \
             navals en profitent aussi (+9 chacun).",
            delta(gain),
            coin_word(gain, cur),
            amortized()
        ),
        InvestmentType::GrainMills => format!(
            "Sur une récolte comme celle-ci, ≈ {} {} par an ; deux fois moins en année de \
             sécheresse{}. Le rendement de chaque moulin baisse un peu à mesure qu'on en ajoute.",
            delta(gain),
            coin_word(gain, cur),
            amortized()
        ),
        InvestmentType::Foundries => format!(
            "≈ {} {} par an au trésor, contre −500 bx de grain (≈ {} au cours du marché).",
            delta(gain),
            coin_word(gain, cur),
            coins(grain_value(500, rate), cur)
        ),
        InvestmentType::Shipyards => format!(
            "Avec {} marchands et {} foires : ≈ {} {} par an, la moitié par mauvais temps{}, \
             plus vite si le commerce grandit.",
            fmt(k.merchants),
            fmt(k.marketplaces),
            delta(gain),
            coin_word(gain, cur),
            amortized()
        ),
        InvestmentType::Soldiers => format!(
            "Chaque recrue : 8 {cur} de solde par an, {} bx à servir au conseil, se bat à {} %. \
             Sans soldats, une attaque tombe sur vos serfs.",
            tenths(b.draft.soldiers),
            b.now.efficiency
        ),
        InvestmentType::Palaces => format!(
            "{} à {} nobles : jusqu'à {} hommes d'armes, ≈ {} à {} {} de taille par an. Le \
             palais lui-même ne rapporte rien : il achète la cour.",
            k.nobles + 1,
            k.nobles + 3,
            fmt((k.nobles + 3) * 20),
            delta(b.now.net.low - b.before.net.low),
            delta(b.now.net.high - b.before.net.high),
            coin_word(b.now.net.high - b.before.net.high, cur)
        ),
    }
}

fn purchase_sheet(t: T, id: Kingdoms, kind: InvestmentType) -> ElementBuilder {
    let k = t.room.game.kingdom(id);
    let cur = k.currency();
    let cost = kind.cost();
    let max = kind.max_investment(k).max(0);
    let b = Board::purchase(k, t.room.game.weather, t.room.draft(id), kind, max);
    let have = owned(k, kind);
    let sub = match kind {
        InvestmentType::Palaces => format!(
            "{} {cur} le dixième · {have} sur 10 · trésor {}",
            fmt(cost),
            coins(k.treasury, cur)
        ),
        InvestmentType::Soldiers => format!(
            "8 {cur} la recrue · {} en armes · vos nobles en mènent {}",
            fmt(have),
            fmt(k.nobles * 20)
        ),
        _ => {
            let none = match kind {
                InvestmentType::Foundries => "aucune",
                _ => "aucun",
            };
            let count = if have > 0 {
                format!("vous en avez {}", fmt(have))
            } else {
                none.to_string()
            };
            format!(
                "{} {cur} · {count} · trésor {}",
                fmt(cost),
                coins(k.treasury, cur)
            )
        }
    };
    let chips = el(El::Div)
        .st([St::DisplayFlex, St::FlexWrap, St::GapXs])
        .append(
            invest_chips(kind)
                .iter()
                .map(|c| Badge::new().text(*c).build()),
        );
    let (verb, one, many) = invest_verb(kind);
    let mut body = vec![
        subtitle(vec![txt(&sub)]),
        intro(&invest_intro(kind, cur)),
        el(El::P)
            .st([St::TextSm, St::TextMuted, St::M0])
            .text(&invest_yield(&b, kind, going_rate(t.room))),
        chips,
    ];
    if max < 1 {
        let reason = if kind == InvestmentType::Soldiers && k.nobles * 20 - k.soldiers < 1 {
            if k.nobles == 1 {
                "Votre seul noble ne peut mener plus de 20 hommes.".to_string()
            } else {
                format!(
                    "Vos {} nobles ne peuvent mener plus de {} hommes.",
                    fmt(k.nobles),
                    fmt(k.nobles * 20)
                )
            }
        } else {
            format!("Il manque {}.", coins(cost - k.treasury, cur))
        };
        body.push(register(&b, Cell::moved_by(Along::Units(kind))));
        body.extend(refused(&reason, format!("{verb} 1 {one}")));
        return Stack::column().gap(Gap::Sm).children(body).build();
    }
    let ch = b.slider().channel;
    let after = |f: &dyn Fn(i32) -> i32| {
        el(El::Span)
            .text(&fmt(f(1)))
            .live_sum(along(ch, 1, 1, max, f))
    };
    let label = match kind {
        InvestmentType::Marketplaces => "Champs",
        InvestmentType::GrainMills => "Moulins",
        InvestmentType::Foundries => "Fonderies",
        InvestmentType::Shipyards => "Chantiers",
        InvestmentType::Soldiers => "Recrues",
        InvestmentType::Palaces => "Dixièmes",
    };
    body.extend([
        line(
            "Trésor après l'achat",
            vec![
                after(&|n| k.treasury - n * cost),
                txt(" "),
                live_coin_word(ch, cur, 1, 1, max, &|n| k.treasury - n * cost),
            ],
        ),
        register(&b, Cell::moved_by(Along::Units(kind))),
        Slider::new()
            .name("invest_amount")
            .id("invest_amount")
            .channel(ch)
            .grouped(fmt)
            .label(label)
            .min(1)
            .max(max)
            .value(1)
            .readout_suffix(
                el(El::Span)
                    .st([St::TextSm, St::TextMuted, St::TabularNums])
                    .append([
                        txt("· "),
                        after(&|n| n * cost),
                        txt(&format!(" {cur} · max {}", fmt(max))),
                    ]),
            )
            .build(),
        cta(vec![
            txt(&format!("{verb} ")),
            el(El::Span).text("1").live_text_grouped(ch),
            txt(" "),
            el(El::Span).live_switch(ch, &[2]).append([
                el(El::Span).text(one),
                hidden_unless(false, el(El::Span).text(many)),
            ]),
        ]),
    ]);
    form(by(room::invest(), t.token, t.code, &[kind as u8 + 1]), body)
}

// ---------------------------------------------------------------------------
// Intendance: the new year on one roll — the register pinned at the top,
// then the market, the rations, the taxes, the purchases, and the act that
// closes the council. Every decision opens a sheet.
// ---------------------------------------------------------------------------

fn intendance_step(t: T, id: Kingdoms) -> ElementBuilder {
    let room = t.room;
    let k = room.game.kingdom(id);
    let seat = room.seat(id);
    let cur = k.currency();
    let b = Board::council(k, room.game.weather, room.draft(id));
    let surplus = k.grain_stocks - k.peasants_grain_needs() - k.soldiers_grain_needs();
    let notice = |spot: Spot| {
        (seat.notice_spot == spot)
            .then(|| seat.notice.clone())
            .flatten()
            .map(|n| Alert::info().message(n).build())
    };

    let market = block(
        heading(
            Icon::Sack,
            "Marché du grain",
            "les étals de l'an",
            Some(figure_aside(
                if surplus < 0 { "manque" } else { "surplus" },
                &fmt(surplus.abs()),
                "bx",
            )),
        ),
        market_rows(t, id),
        notice(Spot::Market),
    );

    let rations = block(
        heading(
            Icon::Bowl,
            "Rations",
            "le pain du peuple et de l'ost",
            Some(figure_aside("il reste", &fmt(b.now.reserve), "bx")),
        ),
        vec![
            ration_row(t, &b, Field::Peasants),
            ration_row(t, &b, Field::Soldiers),
        ],
        None,
    );

    let taxes = block(
        heading(Icon::Coins, "Impôts", "douane, gabelle, taille", None),
        vec![
            tax_row(t, &b, Field::Customs),
            tax_row(t, &b, Field::Sales),
            tax_row(t, &b, Field::Income),
        ],
        None,
    );

    // Pinned to the top of the scroll so every setting is read against it;
    // it bleeds into the page's side padding to hide what scrolls beneath.
    let forecast = el(El::Div)
        .st([
            St::PositionSticky,
            St::Top0,
            St::Z10,
            St::BgApp,
            St::MxNegMd,
            St::PxMd,
            St::PySm,
            St::BorderB,
            St::DisplayFlex,
            St::FlexCol,
            St::GapSm,
        ])
        .append([
            heading(
                Icon::FileText,
                "Prévisionnel",
                &format!("l'an {} tel que vous le réglez", room.game.year + 1),
                None,
            ),
            register(&b, &[]),
        ]);

    let purchases = block(
        heading(
            Icon::Castle,
            "Achats",
            "bâtiments et recrues",
            Some(figure_aside("trésor", &fmt(k.treasury), cur)),
        ),
        vec![el(El::Div)
            .st([St::DisplayGrid, St::GridCols3, St::GapSm, St::PtSm])
            .append(
                (1..=6)
                    .filter_map(InvestmentType::from_number)
                    .map(|kind| tile(t, k, kind)),
            )],
        notice(Spot::Purchases),
    );

    let close = el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapSm, St::PtSm])
        .append([
            el(El::P).st([St::TextSm, St::TextMuted, St::M0]).text(
                "Le conseil est levé : les rations sont servies, les impôts levés, vos achats \
                 faits. Rien ne se rouvre avant l'an prochain.",
            ),
            next("Continuer", t),
        ]);

    Stack::column()
        .gap(Gap::Lg)
        .children([forecast, market, rations, taxes, purchases, close])
        .build()
}

/// A decision block: its heading, its rows, and the notice of the last deal
/// made from it.
fn block(
    heading: ElementBuilder,
    rows: Vec<ElementBuilder>,
    notice: Option<ElementBuilder>,
) -> ElementBuilder {
    let mut children = vec![heading];
    children.extend(rows);
    children.extend(notice);
    el(El::Div)
        .st([
            St::BorderDefault,
            St::RoundedLg,
            St::PMd,
            St::DisplayFlex,
            St::FlexCol,
            St::GapSm,
        ])
        .append(children)
}

/// A section heading: the icon in its pill, the title over its subtitle, a
/// figure on the right.
fn heading(icon: Icon, title: &str, sub: &str, right: Option<ElementBuilder>) -> ElementBuilder {
    let mut children = vec![
        el(El::Div)
            .st([St::Pill, St::BgAccentSubtle, St::TextAccent])
            .append([icon_sized(icon, 16)]),
        el(El::Div).st([St::Flex1, St::MinW0]).append([
            el(El::Div).st([St::FontSemibold]).text(title),
            el(El::Div).st([St::TextXs, St::TextMuted]).text(sub),
        ]),
    ];
    children.extend(right);
    el(El::Div)
        .st([St::DisplayFlex, St::ItemsCenter, St::GapSm])
        .append(children)
}

/// `label` over a figure and its unit, right-aligned.
fn figure_aside(label: &str, figure: &str, unit: &str) -> ElementBuilder {
    el(El::Div)
        .st([St::TextRight, St::TabularNums, St::WhitespaceNowrap])
        .append([
            el(El::Div).st([St::TextXs, St::TextMuted]).text(label),
            el(El::Div).st([St::TextSm]).append([
                el(El::Strong).text(figure),
                el(El::Span).st([St::TextMuted]).text(&format!(" {unit}")),
            ]),
        ])
}

/// One row of a block: a mark on the left, a name over a note, a figure,
/// and the button that opens its sheet.
fn row(
    mark: ElementBuilder,
    name: &str,
    note: Vec<ElementBuilder>,
    figure: Vec<ElementBuilder>,
    button: ElementBuilder,
) -> ElementBuilder {
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapSm,
            St::PySm,
            St::BorderT,
        ])
        .append([
            mark,
            el(El::Div).st([St::Flex1, St::MinW0]).append([
                el(El::Div)
                    .st([St::TextSm, St::FontSemibold, St::WhitespaceNowrap])
                    .text(name),
                el(El::Div)
                    .st([St::TextXs, St::TextMuted, St::TabularNums])
                    .append(note),
            ]),
            el(El::Div)
                .st([St::TextRight, St::TabularNums, St::WhitespaceNowrap])
                .append(figure),
            button,
        ])
}

/// The button of a row: opens `act`.
fn opener(t: T, label: &'static str, act: Action) -> ElementBuilder {
    Button::secondary(label)
        .size(ButtonSize::Sm)
        .on_click(sheet_spec(t, act))
}

/// A big figure over its unit.
fn big(figure: &str, unit: &str) -> Vec<ElementBuilder> {
    vec![
        el(El::Div)
            .st([St::TextLg, St::FontBold, St::LeadingTight])
            .text(figure),
        el(El::Div).st([St::TextXs, St::TextMuted]).text(unit),
    ]
}

/// The two-letter blazon of a kingdom on its colour.
fn blazon(o: Kingdoms) -> ElementBuilder {
    let letters: String = o.name().chars().take(2).collect::<String>().to_uppercase();
    el(El::Div)
        .st([St::Pill, St::TextXs, St::FontBold, St::TextWhite])
        .style(Style::new().background(CURVE_COLORS[o.index()]))
        .text(&letters)
}

/// An icon in a muted pill, the mark of the viewer's own rows.
fn mark(icon: Icon) -> ElementBuilder {
    el(El::Div)
        .st([St::Pill, St::BgMuted, St::TextMuted])
        .append([icon_sized(icon, 16)])
}

/// The market: a row per stall, one for the viewer's sales, one for the
/// land.
fn market_rows(t: T, id: Kingdoms) -> Vec<ElementBuilder> {
    let room = t.room;
    let k = room.game.kingdom(id);
    let mut rows: Vec<ElementBuilder> = sellers(room)
        .into_iter()
        .filter(|&o| o != id)
        .map(|o| {
            let s = room.game.kingdom(o);
            row(
                blazon(o),
                &format!("Grain de la {}", o.name()),
                vec![txt(&format!("{} bx · courtage 10 %", fmt(s.grain_to_sell)))],
                big(&fmt(s.grain_price.min(MAX_GRAIN_PRICE)), "le cent"),
                opener(t, "Acheter", Action::Buy(o)),
            )
        })
        .collect();
    let listed = match k.listing {
        Some((a, p)) => format!("{} bx à l'étal, à {p}", fmt(a)),
        None => "rien à l'étal".to_string(),
    };
    rows.push(row(
        mark(Icon::Wheat),
        "Vos ventes",
        vec![txt(&listed)],
        big(&fmt(k.grain_stocks), "bx en réserve"),
        opener(t, "Vendre", Action::Sell),
    ));
    rows.push(row(
        mark(Icon::Map),
        "Vos terres",
        vec![txt(&format!(
            "{} arpents · {} au plus",
            fmt(k.surface),
            fmt(max_land_sale(k.surface))
        ))],
        big(&LAND_SELL_PRICE.to_string(), "l'arpent"),
        opener(t, "Vendre", Action::Land),
    ));
    rows
}

/// A dial under a row's name: the value on its run, the marks of the bands.
fn dial(value: i32, max: i32, marks: &[i32]) -> ElementBuilder {
    let at = |v: i32| v.clamp(0, max) as f64 / max.max(1) as f64;
    let mut parts: Vec<ElementBuilder> = marks.iter().map(|&m| gauge_tick(at(m))).collect();
    parts.push(
        el(El::Div)
            .st([St::GaugeDot])
            .style(Style::new().set("left", &format!("{:.1}%", at(value) * 100.0))),
    );
    gauge(parts).st([St::MtXs])
}

/// Peuple / Ost: the ration a head, the heads, the dial.
fn ration_row(t: T, b: &Board, field: Field) -> ElementBuilder {
    let k = b.k;
    let (icon, name, per, heads, full, max) = match field {
        Field::Peasants => (
            Icon::Bowl,
            "Peuple",
            "par tête",
            format!("{} âmes", fmt(k.population())),
            Council::PEASANTS_FULL,
            Draft::bounds(k).0,
        ),
        _ => (
            Icon::Helmet,
            "Ost",
            "par homme",
            hommes(k.soldiers),
            Council::SOLDIERS_FULL,
            Draft::bounds(k).1,
        ),
    };
    let value = b.draft.get(field);
    row(
        mark(icon),
        name,
        vec![
            txt(&format!("{per} · {heads}")),
            dial(value, max, &[full, full * 3 / 2]),
        ],
        big(&tenths(value), "bx"),
        opener(t, "Régler", Action::Council(field)),
    )
}

/// A tax: the rate, its expected revenue, the dial.
fn tax_row(t: T, b: &Board, field: Field) -> ElementBuilder {
    let cur = b.k.currency();
    let copy = tax_copy(field);
    let max = match field {
        Field::Customs => Taxes::MAX_CUSTOMS,
        Field::Sales => Taxes::MAX_SALES,
        _ => Taxes::MAX_INCOME,
    };
    let value = b.draft.get(field);
    let marks: Vec<i32> = copy.marks.iter().map(|&(at, _)| at).collect();
    row(
        mark(Icon::Coins),
        copy.label,
        vec![
            el(El::Span).append([
                b.bounds(copy.revenue).figure(true),
                txt(&format!(" {}", coin_word((copy.revenue)(&b.now).high, cur))),
            ]),
            dial(value, max, &marks),
        ],
        big(&value.to_string(), "%"),
        opener(t, "Régler", Action::Council(field)),
    )
}

/// A purchase tile: the icon, what is owned in a medallion, the name, the
/// price. Affordable tiles carry an accent border; the others are dimmed but
/// open all the same.
fn tile(t: T, k: &Kingdom, kind: InvestmentType) -> ElementBuilder {
    let cur = k.currency();
    let affordable = kind.max_investment(k) >= 1;
    let have = owned(k, kind);
    let (name, price, medal) = match kind {
        InvestmentType::Soldiers => ("Hommes d'armes", "8 la recrue".to_string(), fmt(have)),
        InvestmentType::Palaces => (
            "Palais",
            format!("{} le 1/10", fmt(kind.cost())),
            format!("{have}/10"),
        ),
        _ => (
            match kind {
                InvestmentType::Marketplaces => "Champ de foire",
                InvestmentType::GrainMills => "Moulin",
                InvestmentType::Foundries => "Fonderie",
                _ => "Chantier naval",
            },
            format!("{} {cur}", fmt(kind.cost())),
            format!("×{have}"),
        ),
    };
    let mut tokens = vec![
        St::DisplayFlex,
        St::FlexCol,
        St::ItemsCenter,
        St::GapXs,
        St::PSm,
        St::RoundedMd,
        St::TextCenter,
        St::CursorPointer,
        St::BgSurface,
        St::TextDefault,
        St::FontInheritAll,
    ];
    tokens.push(if affordable {
        St::BorderAccent
    } else {
        St::BorderDefault
    });
    if !affordable {
        tokens.push(St::Opacity50);
    }
    el(El::Button)
        .st(tokens)
        .at(At::Type, Av::Button)
        .on(Ev::Click, sheet_spec(t, Action::Invest(kind)))
        .append([
            el(El::Div)
                .st([St::DisplayFlex, St::ItemsCenter, St::GapXs, St::TextAccent])
                .append([
                    icon_sized(invest_icon(kind), 20),
                    Badge::new().text(medal).build(),
                ]),
            el(El::Div)
                .st([St::TextSm, St::FontSemibold, St::LeadingTight])
                .text(name),
            el(El::Div)
                .st([St::TextXs, St::TextMuted, St::TabularNums])
                .text(&price),
        ])
}

/// "champs de foire" → "Champs de foire"
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// The war step: the ost as a stacked bar (what marches where, what stays),
/// then the targets as tiles, the barbarians first. Touching a tile opens
/// that target's sheet ([`war_sheet`]); a tile whose army is ordered says so
/// and reopens its sheet to settle or withdraw it. The armies march once
/// every seigneur has given their orders.
fn war_step(t: T, id: Kingdoms) -> ElementBuilder {
    let room = t.room;
    let k = room.game.kingdom(id);
    let left = room.expeditions_left(id);
    let garrison = room.garrison(id);
    let year = room.game.year;
    let others: Vec<Kingdoms> = room
        .game
        .alive_kingdoms()
        .into_iter()
        .filter(|&o| o != id)
        .collect();

    let title = el(El::Div)
        .st([St::DisplayFlex, St::JustifyBetween, St::ItemsBaseline])
        .append([
            Text::new()
                .variant(TextVariant::Heading3)
                .content("Guerre")
                .build(),
            el(El::Span)
                .st([St::TextSm, St::TextMuted, St::TabularNums])
                .append([
                    el(El::Strong)
                        .st([St::TextDefault])
                        .text(&left.max(0).to_string()),
                    txt(if left == 1 {
                        " expédition encore · "
                    } else {
                        " expéditions encore · "
                    }),
                    el(El::Strong).st([St::TextDefault]).text(&fmt(garrison)),
                    txt(" en garnison"),
                ]),
        ]);

    let tiles = [None]
        .into_iter()
        .chain(others.iter().map(|&o| Some(o)))
        .map(|target| target_tile(t, id, target));
    let hint = if k.soldiers < 1 {
        "Vous n'avez plus d'hommes d'armes."
    } else if year < 3 && left > 0 && garrison > 0 {
        "Une expédition par tranche de 4 nobles, plus une. Les autres royaumes ne peuvent être attaqués qu'à partir de la 3ème année."
    } else if garrison < 1 {
        "Tous vos hommes d'armes sont déjà en campagne. Toucher une armée rouvre son ordre."
    } else if left < 1 {
        "Vos nobles ne peuvent mener davantage d'expéditions cette année. Toucher une armée rouvre son ordre."
    } else {
        "Une expédition par tranche de 4 nobles, plus une. Toucher une cible règle son armée ; les armées marcheront toutes ensemble."
    };
    Stack::column()
        .gap(Gap::Md)
        .children([
            title,
            ost_bar(t, id, None),
            section(
                "Cibles",
                el(El::Div)
                    .st([St::DisplayGrid, St::GridCols3, St::GapSm])
                    .append(tiles),
            ),
            Text::caption(hint).muted().build(),
        ])
        .build()
}

/// The wire byte naming a target: 0 = the barbarians, else the kingdom number.
fn target_byte(target: Option<Kingdoms>) -> u8 {
    target.map_or(0, |o| o.index() as u8 + 1)
}

/// The target the viewer's war sheet is open on, if it still may be.
fn war_target(t: T, me: Option<Kingdoms>) -> Option<(Kingdoms, Option<Kingdoms>)> {
    let id = me?;
    let room = t.room;
    if !room.may_act(id, Step::War) {
        return None;
    }
    let n = room.seat(id).target?;
    let target = match n {
        0 => None,
        n => {
            let o = Kingdoms::from_number(i32::from(n))?;
            (o != id && !room.game.kingdom(o).is_dead).then_some(o)?;
            Some(o)
        }
    };
    Some((id, target))
}

/// A target's tile: the party's colour on top, the garrison it holds (or the
/// army ordered on it), its name and its lands. Dimmed once no more army can
/// be ordered, unless one already is.
fn target_tile(t: T, id: Kingdoms, target: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
    let year = room.game.year;
    let planned = room.planned_on(id, target).map(|(_, e)| e.soldiers);
    let enabled = planned.is_some()
        || (room.expeditions_left(id) > 0
            && room.garrison(id) > 0
            && (target.is_none() || year >= 3));
    let (name, badge, sub) = match target {
        None => (
            "Barbares",
            planned.map_or_else(|| "∞".to_string(), hommes),
            "terres sans fin".to_string(),
        ),
        Some(o) => {
            let k = room.game.kingdom(o);
            (
                o.name(),
                planned.map_or_else(|| format!("{} ⚔", fmt(k.soldiers)), hommes),
                if year < 3 {
                    "dès l'an 3".to_string()
                } else {
                    format!("{} arp.", fmt(k.surface))
                },
            )
        }
    };
    let badge = if planned.is_some() {
        Badge::primary(format!("{badge} ✓"))
    } else {
        Badge::new().text(badge)
    };
    let mut tokens = vec![
        St::DisplayFlex,
        St::FlexCol,
        St::ItemsCenter,
        St::GapXs,
        St::PSm,
        St::RoundedMd,
        St::TextCenter,
        St::BgSurface,
        St::TextDefault,
        St::FontInheritAll,
        St::BorderT3Party,
    ];
    tokens.push(if planned.is_some() {
        St::BorderAccent
    } else {
        St::BorderDefault
    });
    tokens.push(if enabled {
        St::CursorPointer
    } else {
        St::Opacity50
    });
    let mut tile = el(El::Button)
        .st(tokens)
        .style(Style::new().set("--kc", campaign::party_color(target)))
        .at(At::Type, Av::Button)
        .append([
            badge.build(),
            el(El::Div)
                .st([St::TextSm, St::FontSemibold, St::LeadingTight])
                .text(name),
            el(El::Div)
                .st([St::TextXs, St::TextMuted, St::TabularNums])
                .text(&sub),
        ]);
    if enabled {
        tile = tile.on(
            Ev::Click,
            by(room::pick_target(), t.token, t.code, &[target_byte(target)]),
        );
    } else {
        tile = tile.bool_attr(At::Disabled);
    }
    tile
}

/// The ost as a stacked bar over every man of arms: one segment per army
/// ordered, in its target's colour, the garrison hatched. With `live`, the
/// segment of the target being settled follows the sheet's slider (its
/// channel, the men it stands at, its maximum).
fn ost_bar(t: T, id: Kingdoms, live: Option<(Option<Kingdoms>, u16, i32, i32)>) -> ElementBuilder {
    let room = t.room;
    let total = room.game.kingdom(id).soldiers.max(1);
    let fixed: Vec<(Option<Kingdoms>, i32)> = room
        .seat(id)
        .planned
        .iter()
        .filter(|e| live.is_none_or(|(target, ..)| e.target != target))
        .map(|e| (e.target, e.soldiers))
        .collect();
    let others: i32 = fixed.iter().map(|&(_, n)| n).sum();
    let value = live.map_or(0, |(_, _, v, _)| v);
    let marching = others + value;
    let seg = |left: i32, width: i32, target: Option<Kingdoms>| {
        Style::new()
            .set("left", &pct_of(left, total))
            .width(&pct_of(width, total))
            .set("--kc", campaign::party_color(target))
    };
    let square = |target: Option<Kingdoms>| {
        el(El::Span)
            .st([
                St::DisplayInlineBlock,
                St::W05rem,
                St::H05rem,
                St::RoundedSm,
                St::BgParty,
            ])
            .style(Style::new().set("--kc", campaign::party_color(target)))
    };

    let mut segments = Vec::new();
    let mut legend = Vec::new();
    let mut at = 0;
    for &(target, n) in &fixed {
        segments.push(
            el(El::Div)
                .st([St::BarSeg, St::BgParty])
                .style(seg(at, n, target)),
        );
        legend.push(
            el(El::Span)
                .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
                .append([
                    square(target),
                    txt(&format!("{} {}", fmt(n), campaign::party_name(target))),
                ]),
        );
        at += n;
    }
    let (partent, restent) = match live {
        Some((target, ch, v, max)) => {
            let sum = along(ch, v, 1, max, &|n| others + n);
            segments.push(
                el(El::Div)
                    .st([St::BarSeg, St::BgParty])
                    .style(seg(at, v, target))
                    .live_span(
                        LiveSum {
                            base: others,
                            terms: Vec::new(),
                        },
                        sum.clone(),
                        (0, total),
                        (0, total),
                    ),
            );
            segments.push(
                el(El::Div)
                    .st([St::BarSeg, St::BgHatched])
                    .style(seg(marching, total - marching, None))
                    .live_span(
                        sum.clone(),
                        LiveSum {
                            base: total,
                            terms: Vec::new(),
                        },
                        (0, total),
                        (0, total),
                    ),
            );
            legend.push(
                el(El::Span)
                    .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
                    .append([
                        square(target),
                        el(El::Span).text(&fmt(v)).live_text_grouped(ch),
                        txt(&format!(" {}", campaign::party_name(target))),
                    ]),
            );
            (
                el(El::Span).text(&fmt(marching)).live_sum(sum),
                el(El::Span)
                    .text(&fmt(total - marching))
                    .live_remainder_grouped((total - others) as u32, &[ch]),
            )
        }
        None => {
            segments.push(el(El::Div).st([St::BarSeg, St::BgHatched]).style(seg(
                marching,
                total - marching,
                None,
            )));
            (
                el(El::Span).text(&fmt(marching)),
                el(El::Span).text(&fmt(total - marching)),
            )
        }
    };
    legend.push(
        el(El::Span)
            .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
            .append([
                el(El::Span).st([
                    St::DisplayInlineBlock,
                    St::W05rem,
                    St::H05rem,
                    St::RoundedSm,
                    St::BgHatched,
                ]),
                match live {
                    Some((_, ch, ..)) => el(El::Span)
                        .text(&fmt(total - marching))
                        .live_remainder_grouped((total - others) as u32, &[ch]),
                    None => el(El::Span).text(&fmt(total - marching)),
                },
                txt(" garnison"),
            ]),
    );
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapXs])
        .append([
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::JustifyBetween,
                    St::ItemsBaseline,
                    St::TextSm,
                ])
                .append([
                    el(El::Strong).text("Votre ost"),
                    el(El::Span).st([St::TextMuted, St::TabularNums]).append([
                        partent,
                        txt(" partent · "),
                        restent,
                        txt(" restent"),
                    ]),
                ]),
            el(El::Div).st([St::Bar]).append(segments),
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::FlexWrap,
                    St::GapXSm,
                    St::GapYSm,
                    St::TextXs,
                    St::TextMuted,
                    St::TabularNums,
                ])
                .append(legend),
        ])
}

/// One cell of the war sheet's register: a small-caps label over a figure,
/// and a gauge when the figure has one.
fn war_cell(
    label: &str,
    figure: Vec<ElementBuilder>,
    gauge: Option<ElementBuilder>,
) -> ElementBuilder {
    let mut lines = vec![
        el(El::Div)
            .st([
                St::TextXs,
                St::TextUppercase,
                St::TrackingWider,
                St::FontSemibold,
                St::TextMuted,
                St::WhitespaceNowrap,
            ])
            .text(label),
        el(El::Div)
            .st([
                St::TextSm,
                St::FontBold,
                St::LeadingTight,
                St::TrackingTight,
                St::TabularNums,
                St::WhitespaceNowrap,
                St::OverflowHidden,
            ])
            .append(figure),
    ];
    lines.extend(gauge.map(|g| g.st([St::MtXs, St::MbXs])));
    el(El::Div).st([St::MinW0]).append(lines)
}

/// The war sheet of one target: who holds it, the rule of the fight, the ost
/// bar and a register of what the army would take and lose — all following
/// the slider — then the red verb. An army already ordered there reopens
/// with its men, to be settled again or withdrawn.
fn war_sheet(t: T, id: Kingdoms, target: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
    let seat = room.seat(id);
    let k = room.game.kingdom(id);
    let total = k.soldiers.max(1);
    let planned = room.planned_on(id, target);
    let max = room.available(id, target);
    let fc = seat
        .forecast
        .as_ref()
        .filter(|f| f.target == target && f.max == max.max(1))
        .cloned()
        .unwrap_or_else(|| room.war_forecast(id, target));
    let ch = rwire::builder::next_live_channel();
    let value = planned
        .map(|(_, e)| e.soldiers)
        .unwrap_or((max / 2).max(1))
        .clamp(1, max.max(1));

    let (sub, rule, chips): (String, &str, Vec<String>) = match target {
        None => (
            "sans seigneur · des bandes à la mesure de l'ost envoyé".to_string(),
            "Les bandes se lèvent à la mesure de l'ost envoyé et se battent à 90 d'efficacité ; chaque coup porté gagne des terres, sans fin ni seigneur à renverser.",
            vec!["terres sans fin".to_string(), "efficacité 90".to_string()],
        ),
        Some(o) => {
            let d = room.game.kingdom(o);
            (
                format!(
                    "{} · garnison {} ⚔ · efficacité {} · {} arpents",
                    d.player_name,
                    fmt(d.soldiers),
                    d.soldiers_efficiency,
                    fmt(d.surface)
                ),
                "L'armée force la garnison, puis marche le long des terres : les gens rencontrés se rallient une fois sur trois et se battent sinon — serfs et marchands en milice, les nobles avec l'ardeur du royaume. Prendre toutes les terres annexe le royaume.",
                vec![
                    format!("garnison {}", fmt(d.soldiers)),
                    format!("annexion à {}", fmt(d.surface)),
                    "brûle 1 bâtiment sur 3".to_string(),
                ],
            )
        }
    };
    let mut body = vec![
        subtitle(vec![txt(&sub)]),
        intro(rule),
        el(El::Div)
            .st([St::DisplayFlex, St::FlexWrap, St::GapXs])
            .append(chips.iter().map(|c| Badge::new().text(c.clone()).build())),
    ];
    if max < 1 {
        body.extend(refused(
            "Vous n'avez plus d'hommes d'armes à envoyer.",
            "Envoyer",
        ));
        return Stack::column().gap(Gap::Sm).children(body).build();
    }
    body.push(ost_bar(t, id, Some((target, ch, value, max))));

    // The register: the tables follow the slider by interpolation.
    let lookup = |f: &dyn Fn(&empire_lib::front::Forecast) -> i32| -> Vec<i32> {
        fc.rows.iter().map(f).collect()
    };
    let at = |table: &[i32]| -> i32 {
        // The table's value at the slider's initial position.
        let i = fc
            .sent
            .iter()
            .position(|&n| n >= value)
            .unwrap_or(fc.sent.len() - 1);
        table[i]
    };
    let arp_lo = lookup(&|r| r.arpents.0);
    let arp_hi = lookup(&|r| r.arpents.1);
    let lost_lo = lookup(&|r| -r.lost.0);
    let lost_hi = lookup(&|r| -r.lost.1);
    let wins = lookup(&|r| r.victories * 100 / room::FORECAST_DRAWS as i32);
    let range = |lo: &[i32], hi: &[i32], signed: bool, unit: &str| {
        let span = |table: &[i32]| {
            let e = el(El::Span).text(&if signed {
                delta(at(table))
            } else {
                fmt(at(table))
            });
            if signed {
                e.live_lookup_signed(ch, table)
            } else {
                e.live_lookup(ch, table)
            }
        };
        vec![txt("≈ "), span(lo), txt(" … "), span(hi), txt(unit)]
    };
    let forces = match target {
        Some(o) => {
            let d = room.game.kingdom(o).soldiers.max(0);
            let axis = max.max(d).max(1);
            war_cell(
                "Forces",
                vec![
                    el(El::Span).text(&fmt(value)).live_text_grouped(ch),
                    txt(&format!(" contre {}", fmt(d))),
                ],
                Some(gauge([
                    gauge_tick(d as f64 / axis as f64),
                    el(El::Div)
                        .st([St::GaugeSpan, St::BgAccent])
                        .style(Style::new().set("left", "0").width(&pct_of(value, axis)))
                        .live_span(
                            LiveSum {
                                base: 0,
                                terms: Vec::new(),
                            },
                            along(ch, value, 1, max, &|n| n),
                            (0, axis),
                            (0, axis),
                        ),
                ])),
            )
        }
        None => war_cell(
            "Forces",
            vec![
                el(El::Span).text(&fmt(value)).live_text_grouped(ch),
                txt(" contre des bandes"),
            ],
            None,
        ),
    };
    let after = war_cell(
        "Garnison après",
        vec![
            el(El::Span)
                .text(&fmt(max - value))
                .live_remainder_grouped(max as u32, &[ch]),
            txt(" hommes"),
        ],
        Some(gauge([el(El::Div)
            .st([St::GaugeSpan, St::BgWarning])
            .style(
                Style::new()
                    .set("left", "0")
                    .width(&pct_of(max - value, total)),
            )
            .live_span(
                LiveSum {
                    base: 0,
                    terms: Vec::new(),
                },
                along(ch, value, 1, max, &|n| max - n),
                (0, total),
                (0, total),
            )])),
    );
    let last = match target {
        Some(o) => {
            let surface = room.game.kingdom(o).surface;
            let reach = fc
                .sent
                .iter()
                .zip(&arp_hi)
                .find(|(_, &hi)| hi >= surface)
                .map(|(&n, _)| n);
            let figure = match reach {
                Some(n) => vec![el(El::Span).live_switch(ch, &[n as u32]).append([
                    hidden_unless(value < n, el(El::Span).text("hors de portée")),
                    hidden_unless(value >= n, el(El::Span).text("à portée")),
                ])],
                None => vec![txt("hors de portée")],
            };
            war_cell("Annexion", figure, None)
        }
        None => war_cell("Butin", vec![txt("terres seules")], None),
    };
    body.push(
        el(El::Div)
            .st([
                St::DisplayGrid,
                St::GridCols3,
                St::GapXMd,
                St::GapYSm,
                St::PtSm,
                St::BorderT,
            ])
            .append([
                forces,
                war_cell("Terres prises", range(&arp_lo, &arp_hi, false, ""), None),
                war_cell("Hommes perdus", range(&lost_lo, &lost_hi, true, ""), None),
                after,
                war_cell(
                    "Victoire",
                    vec![
                        txt("≈ "),
                        el(El::Span)
                            .text(&at(&wins).to_string())
                            .live_lookup(ch, &wins),
                        txt(" %"),
                    ],
                    None,
                ),
                last,
            ]),
    );

    let foe = match target {
        None => "contre les Barbares".to_string(),
        Some(o) => format!("sur la {}", o.name()),
    };
    body.push(
        Slider::new()
            .name("soldiers")
            .id("soldiers")
            .channel(ch)
            .grouped(fmt)
            .label("Hommes d'armes")
            .min(1)
            .max(max)
            .value(value)
            .mark(max / 2, "½")
            .readout_suffix(
                el(El::Span)
                    .st([St::TextSm, St::TextMuted, St::TabularNums])
                    .append([
                        txt("· restent "),
                        el(El::Span)
                            .text(&fmt(max - value))
                            .live_remainder_grouped(max as u32, &[ch]),
                        txt(&format!(" · max {}", fmt(max))),
                    ]),
            )
            .build(),
    );
    body.push(
        Button::destructive("")
            .size(ButtonSize::Lg)
            .full_width(true)
            .build()
            // One child so the button's flex gap doesn't split the sentence.
            .append([el(El::Span).append([
                txt(if planned.is_some() {
                    "Régler à "
                } else {
                    "Envoyer "
                }),
                el(El::Span).text(&fmt(value)).live_text_grouped(ch),
                txt(&format!(" hommes {foe}")),
            ])]),
    );
    if let Some((i, _)) = planned {
        body.push(
            Button::ghost("Retirer l'armée")
                .full_width(true)
                .build()
                .at(At::Type, Av::Button)
                .on(Ev::Click, by(room::withdraw(), t.token, t.code, &[i as u8])),
        );
    }
    form(
        by(room::attack(), t.token, t.code, &[target_byte(target)]),
        body,
    )
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

/// The six grain figures, 3×2; the last cell is the one the player computes in
/// their head every turn: reserves minus this year's needs.
/// Small-caps rule that opens a group of lines.
fn eyebrow(title: impl Into<Label>) -> ElementBuilder {
    el(El::Div)
        .st([
            St::TextXs,
            St::TextUppercase,
            St::TrackingWider,
            St::FontSemibold,
            St::TextMuted,
            St::PbXs,
        ])
        .text(&title.into())
}

/// A titled group of lines, flat: the rule above, the lines below.
fn section(title: impl Into<Label>, body: ElementBuilder) -> ElementBuilder {
    el(El::Div).append([eyebrow(title), body])
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

fn primary(label: &'static str, spec: HandlerSpec) -> ElementBuilder {
    Button::primary(label)
        .size(ButtonSize::Lg)
        .full_width(true)
        .on_click(spec)
}

/// The war step's final button gives the orders: "Lancer la campagne" once
/// an expedition is ordered; otherwise "Fin du tour", stepping back while an
/// expedition is still possible so the red button in the body reads as the
/// main move.
fn end_turn(t: T, id: Kingdoms) -> ElementBuilder {
    let room = t.room;
    let b = if !room.seat(id).planned.is_empty() {
        Button::primary("Lancer la campagne")
    } else if room.expeditions_left(id) > 0 && room.garrison(id) > 0 {
        Button::secondary("Fin du tour")
    } else {
        Button::primary("Fin du tour")
    };
    b.size(ButtonSize::Lg)
        .full_width(true)
        .on_click(t.act(room::advance()))
}

fn next(label: &'static str, t: T) -> ElementBuilder {
    primary(label, t.act(room::advance()))
}

/// "1 homme" / "12 hommes".
pub fn hommes(n: i32) -> String {
    format!("{} homme{}", fmt(n), if n.abs() == 1 { "" } else { "s" })
}

/// 12345 → "12 345"
pub fn fmt(n: i32) -> String {
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
