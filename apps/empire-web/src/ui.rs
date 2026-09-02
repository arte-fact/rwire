//! Mobile-first views over the shared [`Rooms`]. Pure functions: `page` is
//! re-run for every connection whenever any table changes.

use std::borrow::Cow;

use empire_lib::investments::InvestmentType;
use empire_lib::trade::MAX_GRAIN_PRICE;
use empire_lib::{Kingdom, Kingdoms, PlayerTitle, Weather, KINGDOMS};
use rwire::attr_tokens::{At, Av};
use rwire::{el, El, ElementBuilder, Ev, HandlerSpec, Icon, St, Style};
use rwire_components::{
    Alert, Badge, Button, ButtonIntent, ButtonSize, Card, CardPadding, CopyButton, Drawer,
    DrawerPosition, Gap, Grid, GridColumns, Input, Link, Progress, ProgressIntent, Radio, Slider,
    Spinner, Stack, StackJustify, Stat, StatSize, StatTone, Stepper, Table, TableRow, Text,
    TextVariant,
};

use crate::room::{self, by, invest_fr, Battle, Entry, Room, Rooms, Seat, Stage, Step};

type Label = Cow<'static, str>;

/// Bottom tabs.
const TABS: [&str; 3] = ["Partie", "Royaumes", "Journal"];

/// An open bottom sheet, pinned to the step/year it was opened for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet {
    pub action: u8,
    pub step: u8,
    pub year: u16,
    /// Action argument (Buy: the seller's kingdom number); 0 = none.
    pub arg: u8,
}

/// Step tag used for lobby sheets (no step is active).
const LOBBY_STEP: u8 = 0xFF;

/// Every form that lives in the bottom sheet.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Rename = 1,
    Buy,
    Sell,
    Land,
    Taxes,
    Invest,
}

impl Action {
    fn from_u8(n: u8) -> Option<Action> {
        Some(match n {
            1 => Action::Rename,
            2 => Action::Buy,
            3 => Action::Sell,
            4 => Action::Land,
            5 => Action::Taxes,
            6 => Action::Invest,
            _ => return None,
        })
    }

    fn title(self) -> &'static str {
        match self {
            Action::Rename => "Votre nom",
            Action::Buy => "Acheter du grain",
            Action::Sell => "Vendre du grain",
            Action::Land => "Vendre des terres",
            Action::Taxes => "Taux d'imposition",
            Action::Invest => "Investissements",
        }
    }

    /// The step this action belongs to (`None` = lobby).
    fn step(self) -> Option<Step> {
        Some(match self {
            Action::Rename => return None,
            Action::Buy | Action::Sell | Action::Land => Step::Trade,
            Action::Taxes | Action::Invest => Step::Economy,
        })
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
    let open = sheet.and_then(|sh| open_action(t, me, sh).map(|(id, act)| (id, act, sh.arg)));
    let (content, action) = match tab {
        1 => (kingdoms_tab(t), None),
        2 => (journal_tab(t), None),
        _ => partie(t, me, open.is_some()),
    };
    let overlay = open.map(|(id, act, arg)| {
        let mut body = Vec::new();
        if let Some(notice) = &t.room.seat(id).notice {
            body.push(Alert::info().message(notice.clone()).build());
        }
        body.push(sheet_form(t, id, act, arg));
        Drawer::new()
            .position(DrawerPosition::Bottom)
            .open(true)
            .title(act.title())
            .on_close(crate::close_sheet())
            .content(Stack::column().gap(Gap::Md).children(body).build())
            .build()
    });
    let tabs = (t.room.stage != Stage::Lobby).then_some(tab);
    shell(header(t, me), content, bottom_bar(action, tabs), overlay)
}

/// The sheet's action if it is still valid for this viewer at this moment.
fn open_action(t: T, me: Option<Kingdoms>, sh: Sheet) -> Option<(Kingdoms, Action)> {
    let id = me?;
    let act = Action::from_u8(sh.action)?;
    let room = t.room;
    let valid = match act.step() {
        None => room.stage == Stage::Lobby && sh.step == LOBBY_STEP,
        Some(step) => {
            room.stage == Stage::Playing
                && room.active() == Some(id)
                && room.battle.is_none()
                && room.step == step
                && sh.step == step.index() as u8
                && sh.year == room.game.year as u16
        }
    };
    let k = room.game.kingdom(id);
    let available = match act {
        Action::Buy => seller(room, id, sh.arg).is_some(),
        Action::Sell => k.grain_stocks > 0,
        Action::Land => k.surface > 1,
        _ => true,
    };
    (valid && available).then_some((id, act))
}

/// Handler that opens the sheet for `act` with argument `arg`, pinned to the
/// current step and year.
fn sheet_spec(t: T, act: Action, arg: u8) -> HandlerSpec {
    let step = act.step().map(|s| s.index() as u8).unwrap_or(LOBBY_STEP);
    let year = (t.room.game.year as u16).to_le_bytes();
    crate::open_sheet().with_param_bytes(vec![act as u8, step, year[0], year[1], arg])
}

/// A button that opens the sheet for `act`.
fn opener(t: T, act: Action, label: &'static str, disabled: bool, main: bool) -> ElementBuilder {
    let spec = sheet_spec(t, act, 0);
    let b = if main {
        Button::primary(label)
    } else {
        Button::secondary(label)
    };
    b.full_width(true).disabled(disabled).on_click(spec)
}

/// The form shown inside the sheet.
fn sheet_form(t: T, id: Kingdoms, act: Action, arg: u8) -> ElementBuilder {
    let k = t.room.game.kingdom(id);
    match act {
        Action::Rename => rename_form(t, k),
        Action::Buy => buy_form(t, id, arg),
        Action::Sell => sell_form(t, k),
        Action::Land => land_form(t, k),
        Action::Taxes => taxes_form(t, k),
        Action::Invest => invest_form(t, k, t.room.seat(id)),
    }
}

fn header(t: T, me: Option<Kingdoms>) -> ElementBuilder {
    let room = t.room;
    let (title, sub) = match me {
        Some(id) if room.stage != Stage::Lobby => {
            let k = room.game.kingdom(id);
            (
                format!("{} · {}", k.player_name, k.name()),
                resources_line(k),
            )
        }
        Some(id) => (
            room.game.kingdom(id).full_title(),
            Text::caption(format!(
                "En attente · {} seigneur(s), {} ordinateur(s)",
                room.humans().count(),
                6 - room.humans().count()
            ))
            .muted()
            .build(),
        ),
        None => (
            format!("Table {}", t.code),
            Text::caption("Spectateur").muted().build(),
        ),
    };
    header_bar(
        &title,
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
            figure(k.treasury, k.currency().to_string()),
            figure(k.grain_stocks, "boisseaux".to_string()),
            figure(k.soldiers, "hommes d'armes".to_string()),
        ])
}

fn header_bar(
    title: &str,
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
                el(El::Div).st([St::MinW0]).append([
                    el(El::Strong)
                        .st([St::DisplayBlock, St::Truncate])
                        .text(title),
                    sub,
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
            Stat::new(fmt(room.game.barbarians_surface))
                .label("Terres barbares")
                .build(),
            Stat::new(room.game.alive_kingdoms().len().to_string())
                .label("Royaumes en lice")
                .build(),
        ])
        .build()];
    let mut alive: Vec<Kingdoms> = room.game.alive_kingdoms();
    alive.sort_by_key(|&id| std::cmp::Reverse(room.game.kingdom(id).surface));
    cards.extend(
        alive
            .into_iter()
            .map(|id| kingdom_card(room, id, me == Some(id), most)),
    );
    let fallen: Vec<ElementBuilder> = KINGDOMS
        .into_iter()
        .filter(|&id| room.game.kingdom(id).is_dead)
        .map(|id| {
            Text::caption(format!("{} · annexée", id.name()))
                .muted()
                .build()
        })
        .collect();
    if !fallen.is_empty() {
        cards.push(Stack::column().gap(Gap::Xs).children(fallen).build());
    }
    cards.push(market_section(t, me, false));
    Stack::column().gap(Gap::Md).children(cards).build()
}

/// One kingdom: name and ruler, its land against the largest realm, and the
/// four headcounts that make up its strength.
fn kingdom_card(room: &Room, id: Kingdoms, mine: bool, most: i32) -> ElementBuilder {
    let k = room.game.kingdom(id);
    let figure = |n: i32, label: &'static str| {
        el(El::Span).st([St::WhitespaceNowrap]).append([
            el(El::Strong).st([St::TabularNums]).text(&fmt(n)),
            el(El::Span).st([St::TextMuted]).text(&format!(" {label}")),
        ])
    };
    let mut title = vec![
        el(El::Strong).text(k.name()),
        el(El::Span)
            .st([St::TextMuted])
            .text(&format!(" · {}", k.player_name)),
    ];
    if mine {
        title.push(Badge::new().text("vous").build().st([St::MlSm]));
    }
    if room.is_computer(id) {
        title.push(
            el(El::Span)
                .st([St::TextMuted, St::TextXs])
                .text(" · ordinateur"),
        );
    }
    Card::new()
        .padding(CardPadding::Md)
        .children([
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
                .st([St::MtSm]),
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::FlexWrap,
                    St::GapMd,
                    St::TextSm,
                    St::MtSm,
                ])
                .append([
                    figure(k.nobles, "nobles"),
                    figure(k.soldiers, "soldats"),
                    figure(k.merchants, "marchands"),
                    figure(k.peasants, "serfs"),
                ]),
        ])
        .build()
}

// ---------------------------------------------------------------------------
// Partie tab
// ---------------------------------------------------------------------------

type View = (ElementBuilder, Option<ElementBuilder>);

fn partie(t: T, me: Option<Kingdoms>, sheet_open: bool) -> View {
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
                Some(id) if room.active() == Some(id) => turn(t, id, sheet_open),
                Some(id) if room.game.kingdom(id).is_dead => (
                    Stack::column()
                        .gap(Gap::Md)
                        .children([
                            Alert::error()
                                .title("Votre royaume est tombé")
                                .message("Vous suivez désormais la partie en spectateur.")
                                .build(),
                            waiting(t),
                        ])
                        .build(),
                    None,
                ),
                // Seated players simply wait; only unseated viewers (a lost
                // session, a spectator) are offered the seat-recovery list.
                Some(_) => (waiting(t), None),
                None => (
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
    let invite = Card::new().padding(CardPadding::Md).child(
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::JustifyBetween,
                St::ItemsCenter,
                St::GapSm,
            ])
            .append([
                el(El::Div).append([
                    Text::caption("Code d'invitation").muted().build(),
                    el(El::Strong)
                        .st([St::Text2xl, St::TrackingWidest, St::DisplayBlock])
                        .text(t.code),
                ]),
                CopyButton::new(t.code).build(),
            ]),
    );

    let row_tokens = [
        St::DisplayFlex,
        St::JustifyBetween,
        St::ItemsCenter,
        St::GapSm,
        St::WFull,
        St::PySm,
        St::BorderB,
        St::TextSm,
        St::TextLeft,
    ];
    let seats = KINGDOMS.into_iter().enumerate().map(|(i, id)| {
        let k = room.game.kingdom(id);
        let name = |ruler: &str| {
            el(El::Span).st([St::MinW0]).append([
                el(El::Strong).text(id.name()),
                el(El::Span)
                    .st([St::TextMuted])
                    .text(&format!(" · {ruler}")),
            ])
        };
        match room.seat(id).owner {
            Some(o) if o == t.token => el(El::Div).st(row_tokens).append([
                name(&k.player_name),
                Stack::row()
                    .gap(Gap::Sm)
                    .align_center()
                    .children([
                        Badge::success("Vous").build(),
                        Button::ghost("Nom")
                            .size(ButtonSize::Sm)
                            .on_click(sheet_spec(t, Action::Rename, 0)),
                        Button::ghost("Quitter")
                            .size(ButtonSize::Sm)
                            .on_click(t.act(room::leave())),
                    ])
                    .build(),
            ]),
            Some(_) => el(El::Div)
                .st(row_tokens)
                .append([name(&k.player_name), Badge::warning("Pris").build()]),
            // The whole line is the button: tap a seat to take it.
            None => el(El::Button)
                .st(row_tokens)
                .st([
                    St::BgTransparent,
                    St::BorderNone,
                    St::BorderB,
                    St::Px0,
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
            invite.build(),
            Card::new()
                .padding(CardPadding::Md)
                .children([
                    Text::new()
                        .variant(TextVariant::Heading3)
                        .content("Royaumes")
                        .build(),
                    el(El::Div).append(seats),
                    Text::caption(hint).muted().build().st([St::MtSm]),
                ])
                .build(),
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
                el(El::Span).st([St::Flex1, St::MinW0]).append([
                    el(El::Strong).text(k.name()),
                    el(El::Span)
                        .st([St::TextMuted])
                        .text(&format!(" · {}{note}", k.player_name)),
                ]),
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
                    " · annexée",
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
    let room = t.room;
    let Some(active) = room.active() else {
        return el(El::Div);
    };
    let a = room.game.kingdom(active);
    let computer = room.is_computer(active);
    let order = room.game.alive_kingdoms().into_iter().map(|id| {
        let k = room.game.kingdom(id);
        let badge = match id.index().cmp(&room.turn) {
            std::cmp::Ordering::Less => Badge::default_badge("Joué"),
            std::cmp::Ordering::Equal => Badge::primary("En cours"),
            std::cmp::Ordering::Greater => Badge::default_badge("À venir"),
        };
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
                el(El::Span).st([St::MinW0]).append([
                    el(El::Strong).text(k.name()),
                    el(El::Span).st([St::TextMuted]).text(&format!(
                        " · {}{}",
                        k.player_name,
                        if room.is_computer(id) {
                            " (ordinateur)"
                        } else {
                            ""
                        }
                    )),
                ]),
                badge.build(),
            ])
    });
    let mut turn = vec![
        Text::new()
            .variant(TextVariant::Heading3)
            .content(format!("Au tour de {}", a.player_name))
            .build(),
        Text::caption(if computer {
            "L'ordinateur joue…".to_string()
        } else {
            format!(
                "{} · {}/{}",
                room.step.label(),
                room.step.index() + 1,
                Step::LABELS.len()
            )
        })
        .muted()
        .build(),
    ];
    if !computer {
        turn.push(stepper(room.step));
    }
    Stack::column()
        .gap(Gap::Md)
        .children([
            Card::new().padding(CardPadding::Md).children(turn).build(),
            section("Ordre du tour", el(El::Div).append(order)),
            latest_news(t, 3),
        ])
        .build()
}

/// The live battle, replayed identically on every screen.
fn battle_page(room: &Room, b: &Battle) -> ElementBuilder {
    let f = b.frame();
    let a = room.game.kingdom(b.attack.attacker);
    // Defender: name, headcount on the field, headcount at the start, and the land
    // fought over ("de la Bretagne", its surface).
    let (defender, count, start, land, foe, foe_surface) = match b.attack.target {
        Some(t) => {
            let d = room.game.kingdom(t);
            let (count, start, land) = if f.population_defending {
                (f.defender_peasants, d.peasants, Some(t.name()))
            } else {
                (f.defender_soldiers, b.defender_start, None)
            };
            (
                d.full_title(),
                count,
                start,
                land,
                format!("de la {}", t.name()),
                d.surface,
            )
        }
        None => (
            "Barbares païens".to_string(),
            f.defender_soldiers,
            b.defender_start,
            None,
            "des terres barbares".to_string(),
            room.game.barbarians_surface,
        ),
    };
    let mut items = vec![
        Text::caption("Hommes d'armes restants").muted().build(),
        side(
            a.full_title(),
            f.attacker_soldiers,
            b.attack.soldiers,
            ProgressIntent::Primary,
        ),
        side(defender, count, start, ProgressIntent::Error),
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
    let mut cards = vec![section(
        format!(
            "Bataille · {} → {}",
            b.attack.attacker.name(),
            b.attack.target.map_or("Barbares", |t| t.name())
        ),
        Stack::column().gap(Gap::Sm).children(items).build(),
    )];
    if b.surface_conquered() > 0 {
        cards.push(conquest(b.surface_so_far(), &foe, foe_surface));
    }
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapMd])
        .append(cards)
}

/// The land taken so far, as a share of the defender's ("de la Bretagne"), with the
/// annexation mark.
fn conquest(taken: i32, foe: &str, foe_surface: i32) -> ElementBuilder {
    let whole = foe_surface.max(1);
    let pct = (taken as i64 * 100 / whole as i64) as i32;
    Card::new()
        .padding(CardPadding::Md)
        .children([
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::JustifyBetween,
                    St::ItemsBaseline,
                    St::TextSm,
                ])
                .append([
                    el(El::Span).st([St::TextMuted]).text("Arpents conquis"),
                    el(El::Strong)
                        .st([St::TextLg, St::TabularNums])
                        .text(&fmt(taken)),
                ]),
            Progress::new()
                .value(taken.max(0) as u32)
                .max(whole as u32)
                .thin(true)
                .build()
                .st([St::MtXs]),
            Text::caption(if taken >= whole {
                format!("La totalité {foe} : l'annexion.")
            } else {
                format!("{pct} % {foe} — l'annexion demande {}.", fmt(whole))
            })
            .muted()
            .build()
            .st([St::MtXs]),
        ])
        .build()
}

fn side(name: String, count: i32, start: i32, intent: ProgressIntent) -> ElementBuilder {
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
                .intent(intent)
                .thin(true)
                .build(),
        ])
        .build()
}

// ---------------------------------------------------------------------------
// The active player's turn
// ---------------------------------------------------------------------------

fn turn(t: T, id: Kingdoms, sheet_open: bool) -> View {
    let room = t.room;
    let k = room.game.kingdom(id);
    let seat = room.seat(id);
    let mut items = vec![stepper(room.step)];
    if let (Some(notice), false) = (&seat.notice, sheet_open) {
        items.push(Alert::info().message(notice.clone()).build());
    }
    let (body, action) = match room.step {
        Step::Weather => (weather_step(t, k, seat), Some(next("Au marché", t))),
        Step::Trade => (trade_step(t, id), Some(next("Passer à l'intendance", t))),
        Step::Feed => (
            feed_step(t, k),
            Some(
                Button::primary("Nourrir le royaume")
                    .size(ButtonSize::Lg)
                    .full_width(true)
                    .submits(FEED_FORM),
            ),
        ),
        Step::Report => (
            report_step(t, k, seat),
            Some(next("Passer à l'économie", t)),
        ),
        Step::Economy => (
            economy_step(t, k, seat),
            Some(next("Passer à la guerre", t)),
        ),
        Step::War => (war_step(t, id), Some(end_turn(t, id))),
    };
    items.push(body);
    (Stack::column().gap(Gap::Md).children(items).build(), action)
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
const DELAYS: [St; 8] = [
    St::Delay1,
    St::Delay2,
    St::Delay3,
    St::Delay4,
    St::Delay5,
    St::Delay6,
    St::Delay7,
    St::Delay8,
];

/// Slots a count-up occupies before its formatted value takes over (1.2s).
const COUNT_SLOTS: usize = 4;

fn delay(slot: usize) -> St {
    DELAYS[slot.min(DELAYS.len() - 1)]
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
                .st([St::PositionAbsolute, St::Inset0, St::CountUp, delay(slot)])
                .style(
                    Style::new()
                        .set("--n", &from.to_string())
                        .set("--to", &to.to_string()),
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

/// Full-screen report: a tap anywhere advances; the bottom CTA is a reminder.
fn tap_through(t: T, hint: &'static str, slot: usize, body: Vec<ElementBuilder>) -> ElementBuilder {
    let mut children = body;
    children.push(reveal(
        slot,
        Text::caption(hint)
            .muted()
            .build()
            .st([St::TextCenter, St::PtMd]),
    ));
    el(El::Div)
        .st([St::CursorPointer, St::SelectNone])
        .on(Ev::Click, t.act(room::advance()))
        .append(children)
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
fn sky(weather: Weather, year: i32) -> ElementBuilder {
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
        .append([el(El::Div).st([St::PositionRelative]).append([
            reveal(
                0,
                el(El::Div)
                    .st([St::Text4xl, St::FontBold, St::LeadingNone])
                    .text(&format!("An {year}")),
            ),
            reveal(
                1,
                el(El::Div)
                    .st([St::TextLg, St::MtSm])
                    .text(weather.sentence()),
            ),
        ])])
}

/// Saison: the year's sky, then the harvest ledger line by line, verdict last.
fn weather_step(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let game = &t.room.game;
    let sky = sky(game.weather, game.year);

    let needs = k.peasants_grain_needs() + k.soldiers_grain_needs();
    let balance = k.grain_stocks - needs;
    let (verdict, tone) = if balance < 0 {
        ("Il manque", St::TextError)
    } else {
        ("Il reste", St::TextSuccess)
    };
    let ledger = el(El::Div).st([St::PxXs]).append([
        ledger_row(
            2,
            el(El::Span).text("Récolte"),
            count_up(0, k.grain_harvest, 2).st([St::Text2xl]),
            None,
        ),
        ledger_row(
            COUNT_SLOTS,
            aside("Mangés par les rats", &format!("{} %", k.rats_loss_rate)),
            signed(-seat.rats),
            None,
        ),
        ledger_row(
            5,
            el(El::Span).text("Réserves"),
            el(El::Span).text(&fmt(k.grain_stocks)),
            None,
        ),
        ledger_row(
            6,
            aside("Besoins de l'an", "peuple + ost"),
            el(El::Span).text(&fmt(needs)),
            None,
        ),
        reveal(
            7,
            el(El::Div)
                .st([
                    St::DisplayFlex,
                    St::JustifyBetween,
                    St::ItemsBaseline,
                    St::GapSm,
                    St::PtMd,
                ])
                .append([
                    el(El::Span).text(verdict),
                    el(El::Span)
                        .st([tone, St::TextLg, St::FontBold, St::TabularNums])
                        .text(&format!("{} boisseaux", fmt(balance.abs()))),
                ]),
        ),
    ]);

    tap_through(
        t,
        "Touchez l'écran pour aller au marché",
        7,
        vec![sky, ledger],
    )
}

/// Commerce: one-line reminder of the season, the six figures, the market as
/// a list of sellers with a buy button each, and the two (rare) sales.
fn trade_step(t: T, id: Kingdoms) -> ElementBuilder {
    let game = &t.room.game;
    let k = game.kingdom(id);
    let reminder = el(El::P)
        .st([St::TextSm, St::TextMuted])
        .text(&format!(
            "An {} · {} · récolte ",
            game.year,
            game.weather.sentence().trim_end_matches(['.', '!'])
        ))
        .append([
            el(El::Strong)
                .st([St::TextDefault, St::TabularNums])
                .text(&fmt(k.grain_harvest)),
            el(El::Span).text(" boisseaux"),
        ]);
    Stack::column()
        .gap(Gap::Md)
        .children([
            reminder,
            resources(k),
            market_section(t, Some(id), true),
            two(
                opener(
                    t,
                    Action::Sell,
                    "Vendre du grain",
                    k.grain_stocks < 1,
                    false,
                ),
                opener(t, Action::Land, "Vendre des terres", k.surface < 2, false),
            ),
        ])
        .build()
}

/// Two equal-width buttons side by side.
fn two(a: ElementBuilder, b: ElementBuilder) -> ElementBuilder {
    Grid::new()
        .columns(GridColumns::Fixed2)
        .gap(Gap::Sm)
        .children([a, b])
        .build()
}

/// Every kingdom currently selling grain.
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

/// The seller a Buy sheet was opened for (kingdom number in the sheet arg),
/// if it is still someone else with grain on the market.
fn seller(room: &Room, me: Kingdoms, n: u8) -> Option<Kingdoms> {
    let o = Kingdoms::from_number(i32::from(n))?;
    (o != me && sellers(room).contains(&o)).then_some(o)
}

/// Every current listing on the grain market, the viewer's own included. When
/// `buying`, each foreign listing has its own "Acheter" that opens the sheet
/// on that seller.
fn market_section(t: T, viewer: Option<Kingdoms>, buying: bool) -> ElementBuilder {
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
            "{} bx à {}",
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
        } else if buying {
            row.push(
                Button::primary("Acheter")
                    .size(ButtonSize::Sm)
                    .on_click(sheet_spec(t, Action::Buy, (o.index() + 1) as u8)),
            );
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

/// Buy from one seller: the offer is recalled, only the amount is chosen.
fn buy_form(t: T, id: Kingdoms, arg: u8) -> ElementBuilder {
    let room = t.room;
    let Some(o) = seller(room, id, arg) else {
        return Text::body("Cette offre n'est plus sur le marché.")
            .muted()
            .build();
    };
    let s = room.game.kingdom(o);
    let price = s.grain_price.min(MAX_GRAIN_PRICE);
    let max = s.grain_to_sell.clamp(1, 500);
    form(
        by(room::buy_grain(), t.token, t.code, &[(o.index() + 1) as u8]),
        [
            Text::body(format!(
                "La {} ({}) vend {} boisseaux à {} {} pièce, courtage 10 % compris.",
                o.name(),
                s.player_name,
                fmt(s.grain_to_sell),
                price,
                id.currency()
            ))
            .muted()
            .build(),
            slider("buy_amount", "Boisseaux", 1, max, 100.min(max), "boisseaux"),
            Button::primary("Acheter").full_width(true).build(),
        ],
    )
}

fn sell_form(t: T, k: &Kingdom) -> ElementBuilder {
    let stocks = k.grain_stocks.max(1);
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
            Button::primary("Mettre en vente").full_width(true).build(),
        ],
    )
}

fn land_form(t: T, k: &Kingdom) -> ElementBuilder {
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
            Button::primary("Vendre aux Barbares")
                .full_width(true)
                .build(),
        ],
    )
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

const FEED_FORM: &str = "feed";

/// Intendance: both sliders in the page, with the 100/150/200 % marks that
/// decide the year, a consequence sentence under each, and the budget line.
/// The bottom-bar CTA submits this form.
fn feed_step(t: T, k: &Kingdom) -> ElementBuilder {
    let stocks = k.grain_stocks.max(0);
    let needs = k.peasants_grain_needs();
    let army = k.soldiers_grain_needs();
    // Beyond 2× the people's needs immigration barely grows; beyond 1.5× the
    // army's needs efficiency is already maxed — so the sliders stop there.
    let peasants_max = (needs * 2).min(stocks).max(0);
    let soldiers_max = (army * 3 / 2).min(stocks).max(0);
    let peasants = needs.min(peasants_max);
    let soldiers = army.min(soldiers_max);
    let (ch_p, ch_s) = (
        rwire::builder::next_live_channel(),
        rwire::builder::next_live_channel(),
    );

    let people = Card::new().padding(CardPadding::Md).children([
        ration_slider(
            "peasants",
            ch_p,
            format!("Peuple · {} habitants", fmt(k.population())),
            peasants_max,
            peasants,
            needs,
            &[
                (needs, "100 %"),
                (needs * 3 / 2, "150 %"),
                (needs * 2, "200 %"),
            ],
        ),
        consequences(
            ch_p,
            peasants,
            &[needs / 2, needs, needs * 3 / 2 + 1],
            &[
                "Famine : des sujets meurent de faim, d'autres de malnutrition.",
                "Mal nourri : la malnutrition fait des victimes.",
                "Nourri : le peuple survit, personne n'immigre.",
                "Bien nourri : les étrangers immigrent, des nobles s'installent.",
            ],
        ),
    ]);
    let ost = Card::new().padding(CardPadding::Md).children([
        ration_slider(
            "soldiers",
            ch_s,
            format!("Ost · {} hommes", fmt(k.soldiers)),
            soldiers_max,
            soldiers,
            army,
            &[(army, "100 %"), (army * 3 / 2, "150 %")],
        ),
        consequences(
            ch_s,
            soldiers,
            &[army / 2, army, army * 3 / 2],
            &[
                "Affamé, l'ost perd des hommes et déserte.",
                "Rations réduites : des hommes désertent.",
                "L'ost combattra à pleine force ; mieux encore à 150 %.",
                "L'ost combattra à 150 %, son maximum.",
            ],
        ),
    ]);

    let given = peasants + soldiers;
    let budget = Card::new().padding(CardPadding::Md).children([
        el(El::Div)
            .st([St::DisplayFlex, St::JustifyBetween, St::TextSm])
            .append([
                el(El::Span).st([St::TextMuted]).text("Distribué"),
                el(El::Span)
                    .st([St::TextMuted])
                    .text("Reste en réserve ")
                    .append([
                        el(El::Strong)
                            .st([St::TextDefault, St::TabularNums])
                            .text(&fmt(stocks - given))
                            .live_remainder_grouped(stocks as u32, &[ch_p, ch_s]),
                        el(El::Span).text(" bx"),
                    ]),
            ]),
        el(El::Div)
            .st([
                St::WFull,
                St::H05rem,
                St::RoundedFull,
                St::BgMuted,
                St::OverflowHidden,
                St::MtSm,
            ])
            .append([el(El::Div)
                .st([St::HFull, St::BgAccent, St::RoundedFull])
                .style(Style::new().width(&format!(
                    "{:.1}%",
                    if stocks > 0 {
                        given as f64 / stocks as f64 * 100.0
                    } else {
                        0.0
                    }
                )))
                .live_remainder_fill(stocks as u32, &[ch_p, ch_s])]),
        el(El::P)
            .st([St::TextSm, St::MtXs])
            .live_remainder_switch(stocks as u32, &[ch_p, ch_s], &[0])
            .append([
                hidden_unless(
                    given > stocks,
                    el(El::Span)
                        .st([St::TextError])
                        .text("Plus de grain que de réserves : l'ost sera servi en dernier."),
                ),
                hidden_unless(given <= stocks, el(El::Span)),
            ]),
    ]);

    form(
        t.act(room::feed()),
        [people.build(), ost.build(), budget.build()],
    )
    .at_str(At::Id, FEED_FORM)
    .st([St::GapMd])
}

/// A grain slider whose readout also shows the value as a percentage of `needs`.
fn ration_slider(
    name: &'static str,
    channel: u16,
    label: String,
    max: i32,
    value: i32,
    needs: i32,
    marks: &[(i32, &'static str)],
) -> ElementBuilder {
    let pct = if needs > 0 { value * 100 / needs } else { 0 };
    let mut s = Slider::new()
        .name(name)
        .id(name)
        .channel(channel)
        .grouped(fmt)
        .label(label)
        .unit("bx ·")
        .min(0)
        .max(max)
        .value(value)
        .readout_suffix(
            el(El::Span)
                .st([St::TextSm, St::TextMuted, St::TabularNums])
                .append([
                    el(El::Span).text(&pct.to_string()).live_scaled(
                        channel,
                        100,
                        needs.max(1) as u32,
                    ),
                    el(El::Span).text(" %"),
                ]),
        );
    for &(at, label) in marks {
        s = s.mark(at, label);
    }
    s.build()
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

/// Peuple: the census counts from last year's headcount to this year's while
/// the causes appear one by one, each with a bar; the ost closes with its
/// efficiency gauge and the chronicle sentence stays, in italics.
fn report_step(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let Some(d) = &seat.demo else {
        return el(El::Div);
    };
    let delta = room::population_delta(d);
    let after = k.total_population();
    let before = after - delta;

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
                el(El::Div).st([St::MtSm]).append([match delta {
                    n if n > 0 => Badge::success(format!("+{} sujets", fmt(n))),
                    n if n < 0 => Badge::error(format!("−{} sujets", fmt(-n))),
                    _ => Badge::default_badge("population stable"),
                }
                .build()]),
            ),
        ]);

    let causes = [
        (d.births, "Naissances"),
        (d.immigrants, "Étrangers venus s'installer"),
        (-d.disease_victims, "Morts de maladie"),
        (-d.malnutrition_victims, "Morts de faim"),
        (-d.starvation_victims, "Morts de misère"),
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

    let efficiency = d.soldiers_efficiency * 10;
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
                room::delta_verb(delta),
                fmt(delta.abs())
            )),
    ));

    tap_through(
        t,
        "Touchez l'écran pour passer à l'économie",
        slot + 1,
        vec![census, el(El::Div).st([St::PxXs]).append(rows)],
    )
}

/// Économie: one-line reminder of the census, the year's income in five
/// lines (the three taxes share one), then Investir and the tax rates.
fn economy_step(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let mut items = Vec::new();
    if let Some(d) = &seat.demo {
        let delta = room::population_delta(d);
        items.push(
            el(El::P)
                .st([St::TextSm, St::TextMuted])
                .text("Peuple ")
                .append([
                    signed(delta).st([St::FontSemibold, St::TabularNums]),
                    el(El::Span).text(&format!(
                        " sujets · {} habitants · ost à ",
                        fmt(k.total_population())
                    )),
                    el(El::Strong)
                        .st([St::TextDefault, St::TabularNums])
                        .text(&format!("{} %", d.soldiers_efficiency * 10)),
                ]),
        );
    }
    if let Some(e) = &seat.eco {
        let mut rows = vec![
            (
                "Champs de foire",
                fmt(k.marketplaces),
                e.marketplaces_profits,
            ),
            ("Moulins", fmt(k.grain_mills), e.grain_mills_profits),
            ("Fonderies", fmt(k.foundries), e.foundries_profits),
        ];
        if k.shipyards > 0 {
            rows.push(("Chantiers navals", fmt(k.shipyards), e.shipyards_profits));
        }
        rows.push(("Hommes d'armes", fmt(k.soldiers), -e.soldiers_maintenance));
        rows.push((
            "Taxes",
            format!(
                "{} · {} · {} %",
                k.immigration_taxes, k.commercial_taxes, k.income_taxes
            ),
            e.immigration_taxes_profits + e.commercial_taxes_profits + e.income_taxes_profits,
        ));
        let mut tbl = Table::new()
            .headers(["Poste", "Nb", "Profit"])
            .striped(true);
        for (label, count, profit) in rows {
            tbl = tbl.row(TableRow::new().cells([label.to_string(), count, fmt(profit)]));
        }
        items.push(section(
            format!("Revenus · {} {}", fmt(e.net()), k.currency()),
            tbl.build(),
        ));
    }
    items.push(two(
        opener(t, Action::Invest, "Investir", false, true),
        opener(t, Action::Taxes, "Taux d'imposition", false, false),
    ));
    Stack::column().gap(Gap::Md).children(items).build()
}

fn taxes_form(t: T, k: &Kingdom) -> ElementBuilder {
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
            Button::primary("Promulguer").full_width(true).build(),
        ],
    )
}

fn invest_form(t: T, k: &Kingdom, seat: &Seat) -> ElementBuilder {
    let kind = seat.invest_kind.unwrap_or(InvestmentType::Marketplaces);
    let max = kind.max_investment(k).max(0);
    let ch = rwire::builder::next_live_channel();
    let kinds = (1..=6)
        .filter_map(|n| InvestmentType::from_number(n).map(|k| (n, k)))
        .map(|(n, other)| {
            let cap = other.max_investment(k).max(0);
            choice(
                Radio::new()
                    .name("kind")
                    .value(n.to_string())
                    .checked(other == kind)
                    .on_change(t.act(room::pick_investment())),
                &capitalize(invest_fr(other)),
                None,
                format!("{} {} · max {}", fmt(other.cost()), k.currency(), fmt(cap)),
                cap > 0,
            )
        });
    let amount = 1.min(max);
    form(
        t.act(room::invest()),
        [
            Text::caption(format!("Trésor : {} {}", fmt(k.treasury), k.currency()))
                .muted()
                .build(),
            el(El::Div).append(kinds),
            Slider::new()
                .name("invest_amount")
                .id("invest_amount")
                .channel(ch)
                .grouped(fmt)
                .label(capitalize(invest_fr(kind)))
                .min(0)
                .max(max)
                .value(amount)
                .readout_suffix(
                    el(El::Span)
                        .st([St::TextSm, St::TextMuted, St::TabularNums])
                        .append([
                            el(El::Span).text("· "),
                            el(El::Span)
                                .text(&fmt(amount * kind.cost()))
                                .live_scaled_grouped(ch, kind.cost().max(0) as u32, 1),
                            el(El::Span).text(&format!(" {}", k.currency())),
                        ]),
                )
                .build(),
            Button::primary("Investir")
                .full_width(true)
                .disabled(max < 1)
                .build(),
        ],
    )
}

/// "champs de foire" → "Champs de foire"
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

const WAR_FORM: &str = "war";

/// The war step is one form: the vassal-lands table doubles as the target
/// selector (a radio per row), the troop slider sits under it and the red
/// button launches the expedition. Nothing left to send → the table alone.
fn war_step(t: T, id: Kingdoms) -> ElementBuilder {
    let room = t.room;
    let k = room.game.kingdom(id);
    let left = room.seat(id).attacks_left;
    let year = room.game.year;
    let ready = left > 0 && k.soldiers > 0;

    let status = if k.soldiers < 1 {
        "Vous n'avez plus d'hommes d'armes.".to_string()
    } else if left < 1 {
        "Vos nobles ne peuvent mener davantage d'expéditions cette année.".to_string()
    } else {
        format!("{left} expédition(s) possible(s) · une par tranche de 4 nobles, plus une")
    };
    let note = if year < 3 {
        "Les autres royaumes ne peuvent être attaqués qu'à partir de la 3ème année."
    } else {
        "Conquérir toutes les terres d'un royaume l'annexe : ses serfs deviennent les vôtres."
    };
    let target = |n: usize, checked: bool, enabled: bool| {
        Radio::new()
            .name("target")
            .value(n.to_string())
            .checked(checked && enabled)
            .disabled(!enabled)
            .build()
    };
    let mut rows = vec![choice(
        target(0, true, ready),
        "Barbares",
        None,
        format!("{} arpents", fmt(room.game.barbarians_surface)),
        ready,
    )];
    for other in room.game.alive_kingdoms().into_iter().filter(|&o| o != id) {
        let o = room.game.kingdom(other);
        let enabled = ready && year >= 3;
        rows.push(choice(
            target(other.index() + 1, false, enabled),
            o.name(),
            Some(&o.player_name),
            format!("{} arpents · {} soldats", fmt(o.surface), fmt(o.soldiers)),
            enabled,
        ));
    }
    let targets = Card::new().padding(CardPadding::Md).children([
        Text::new()
            .variant(TextVariant::Heading3)
            .content("Terres vassales")
            .build(),
        Text::caption(status).muted().build(),
        el(El::Div).st([St::MtSm]).append(rows),
        Text::caption(note).muted().build().st([St::MtSm]),
    ]);
    if !ready {
        return targets.build();
    }

    let ch = rwire::builder::next_live_channel();
    let sent = (k.soldiers / 2).max(1);
    let troops = Card::new().padding(CardPadding::Md).children([
        Slider::new()
            .name("soldiers")
            .id("soldiers")
            .channel(ch)
            .grouped(fmt)
            .label(format!("Hommes d'armes · {} en armes", fmt(k.soldiers)))
            .unit("hommes")
            .min(1)
            .max(k.soldiers)
            .value(sent)
            .mark(k.soldiers / 2, "½")
            .build(),
        el(El::P)
            .st([St::TextSm, St::TextMuted, St::MtXs])
            .text("Restent en garnison ")
            .append([
                el(El::Strong)
                    .st([St::TextDefault, St::TabularNums])
                    .text(&(k.soldiers - sent).to_string())
                    .live_remainder(k.soldiers as u32, &[ch]),
                el(El::Span).text(" hommes"),
            ]),
        Button::destructive("")
            .full_width(true)
            .build()
            .st([St::MtSm])
            // One child so the button's flex gap doesn't split the sentence.
            .append([el(El::Span).append([
                el(El::Span).text("Attaquer avec "),
                el(El::Span).text(&sent.to_string()).live_text(ch),
                el(El::Span).text(" hommes"),
            ])]),
    ]);
    form(t.act(room::attack()), [targets.build(), troops.build()])
        .at_str(At::Id, WAR_FORM)
        .st([St::GapMd])
}

/// One line of a radio list: radio · label (· sub) · figures. The whole line
/// is the `<label>`, so tapping anywhere on it selects.
fn choice(
    radio: ElementBuilder,
    label: &str,
    sub: Option<&str>,
    figures: String,
    enabled: bool,
) -> ElementBuilder {
    el(El::Label)
        .st([
            St::DisplayFlex,
            St::ItemsCenter,
            St::GapSm,
            St::PySm,
            St::BorderB,
            St::TextSm,
        ])
        .st(if enabled {
            [St::CursorPointer]
        } else {
            [St::Opacity50]
        })
        .append([
            radio,
            el(El::Span).st([St::Flex1, St::MinW0]).append([
                el(El::Strong).text(label),
                match sub {
                    Some(r) => el(El::Span).st([St::TextMuted]).text(&format!(" · {r}")),
                    None => el(El::Span),
                },
            ]),
            el(El::Span)
                .st([St::TextMuted, St::TabularNums, St::WhitespaceNowrap])
                .text(&figures),
        ])
}

// ---------------------------------------------------------------------------
// Shared pieces
// ---------------------------------------------------------------------------

/// The six grain figures, 3×2; the last cell is the one the player computes in
/// their head every turn: reserves minus this year's needs.
fn resources(k: &Kingdom) -> ElementBuilder {
    let balance = k.grain_stocks - k.peasants_grain_needs() - k.soldiers_grain_needs();
    let stat = |value: String, label: &'static str| {
        Stat::new(value)
            .label(label)
            .size(StatSize::Sm)
            .build()
            .st([St::BorderT, St::PtXs])
    };
    let (verdict, tone, sign) = if balance < 0 {
        ("Manque", StatTone::Error, "−")
    } else {
        ("Surplus", StatTone::Success, "+")
    };
    Grid::new()
        .columns(GridColumns::Fixed3)
        .gap(Gap::Sm)
        .children([
            stat(fmt(k.grain_harvest), "Récolte"),
            stat(fmt(k.grain_stocks), "Réserves"),
            stat(format!("{} %", k.rats_loss_rate), "Rats"),
            stat(fmt(k.peasants_grain_needs()), "Besoins peuple"),
            stat(fmt(k.soldiers_grain_needs()), "Besoins ost"),
            Stat::new(format!("{sign}{}", fmt(balance.abs())))
                .label(verdict)
                .size(StatSize::Sm)
                .tone(tone)
                .build()
                .st([St::BorderT, St::PtXs]),
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

fn form(
    on_submit: HandlerSpec,
    children: impl IntoIterator<Item = ElementBuilder>,
) -> ElementBuilder {
    el(El::Form)
        .st([St::DisplayFlex, St::FlexCol, St::GapSm])
        .on(Ev::Submit, on_submit)
        .append(children)
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
        .grouped(fmt)
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

/// "Fin du tour" steps back while an expedition is still possible, so the red
/// button in the body reads as the main move.
fn end_turn(t: T, id: Kingdoms) -> ElementBuilder {
    let k = t.room.game.kingdom(id);
    let ready = t.room.seat(id).attacks_left > 0 && k.soldiers > 0;
    let b = if ready {
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
