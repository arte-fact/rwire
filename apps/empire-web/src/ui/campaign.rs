//! La Campagne: the year's fronts told one after the other. The order of
//! battle with the next front blinking, then the front itself — every army's
//! gauge against the garrison, then each army's march along its share of the
//! realm with what it takes as it goes — then its verdict, then the schema
//! again; once every front is told, the schema stays and the year may turn.
//! Every seat reads on its own clock: the telling runs by itself, a tap
//! skips to the next screen, and only the last screen waits for the reader.
//! Before a front is fought, the order of battle fades everything that is
//! not on it. The fronts' outcomes reach the map only once everyone has
//! read; a viewer with no seat is shown the campaign fully told.

use empire_lib::campaign::Fought;
use empire_lib::front::{Army, People, Round, Spoils, Stand};

use super::*;
use crate::room::{buildings_fr, goods_fr, lost_fr, people_fr, Replay, Side, Staging};

/// A side of the war: a kingdom, or the barbarian lands.
type Party = Option<Kingdoms>;

/// The colour a party keeps throughout the app (the land curve's).
pub(super) fn party_color(p: Party) -> &'static str {
    p.map_or("var(--j)", |k| CURVE_COLORS[k.index()])
}

pub(super) fn party_name(p: Party) -> &'static str {
    p.map_or("Barbares", Kingdoms::name)
}

/// "la Castille" / "les Barbares".
fn party_the(p: Party) -> String {
    match p {
        Some(k) => format!("la {}", k.name()),
        None => "les Barbares".to_string(),
    }
}

/// Whether `me` stood on this front — its figures are theirs to read. The
/// others hear who marched on whom and what land changed hands, never how
/// many men: a campaign told in full would be a free scout's report on
/// everyone every year.
fn on_field(b: &Fought, me: Option<Kingdoms>) -> bool {
    me.is_some_and(|m| b.target == Some(m) || b.armies().iter().any(|a| a.attacker == m))
}

/// "10 008 arpents" on the viewer's fronts, "≈ 10 000 arpents" elsewhere.
fn arpents_told(n: i32, known: bool) -> String {
    if known {
        format!("{} arpents", fmt(n))
    } else {
        format!("≈ {} arpents", fmt(arpents_heard(n)))
    }
}

/// "12 hommes", or "? hommes" for what stays inside the walls.
fn hommes_heard(n: i32, known: bool) -> String {
    if known {
        hommes(n)
    } else {
        "? hommes".to_string()
    }
}

/// Fixed height of a row in the order of battle, in the SVG's units (one row
/// is 4.5rem; the lines are drawn against a stretched viewBox).
const OB_ROW: f64 = 10.0;

fn mono(text: &str) -> ElementBuilder {
    el(El::Span)
        .st([St::FontMono, St::TextXs, St::TextMuted, St::TabularNums])
        .text(text)
}

fn pct(n: i64, of: i64) -> String {
    format!("{:.2}%", n.max(0) as f64 * 100.0 / of.max(1) as f64)
}

pub(super) fn campaign_page(t: T, me: Option<Kingdoms>) -> View {
    let room = t.room;
    let replay = room.replay(me);
    let body = match replay.staging {
        Staging::Schema(_) | Staging::Done => schema(room, replay, me),
        Staging::Fight(_) => battle(room, replay, me),
        Staging::Verdict(_) => verdict(room, replay, me),
    };
    // A seated player taps their replay on; once it is told, only a seigneur
    // who has not yet continued.
    let taps = match me {
        Some(id) => !replay.done() || room.may_continue(id),
        None => false,
    };
    let line = status(room, replay, me, taps);
    if taps {
        View::tap(body, line, t.act(room::tap_campaign()))
    } else {
        View::new(body, Some(line))
    }
}

/// The line under the screen: what is happening and what a tap does, or who
/// is still reading. It blinks while a front moves.
fn status(room: &Room, replay: Replay, me: Option<Kingdoms>, taps: bool) -> ElementBuilder {
    let n = room.battles.len();
    let text = match replay.staging {
        Staging::Done if taps => "Touchez l'écran pour finir l'année".to_string(),
        Staging::Done => {
            let reading: Vec<&str> = room.still_reading().map(|id| id.name()).collect();
            format!(
                "L'an s'achève quand tous auront continué · {}",
                reading.join(", ")
            )
        }
        Staging::Verdict(_) => format!(
            "Front {}/{n} tranché · touchez pour passer",
            replay.current + 1
        ),
        Staging::Fight(_) => {
            let b = &room.battles[replay.current];
            let armies = &b.result.armies;
            let names: Vec<String> = armies
                .iter()
                .map(|a| format!("la {}", a.attacker.name()))
                .collect();
            let (verb, plural) = if replay.frame(b).garrison == 0 {
                ("pille", "pillent")
            } else {
                ("marche sur", "marchent sur")
            };
            let land = if me.is_some() && me == b.target {
                "vos terres".to_string()
            } else {
                party_the(b.target)
            };
            format!(
                "{} {} {land} · touchez pour abréger",
                cap(&names.join(" et ")),
                if names.len() > 1 { plural } else { verb }
            )
        }
        Staging::Schema(_) => format!("Front {}/{n} · touchez pour passer", replay.current + 1),
    };
    let dot = matches!(replay.staging, Staging::Fight(_)).then(|| {
        el(El::Span).st([
            St::W05rem,
            St::H05rem,
            St::RoundedFull,
            St::BgError,
            St::Blink,
            St::FlexShrink0,
        ])
    });
    hint_line(dot, &text)
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// A · the order of battle
// ---------------------------------------------------------------------------

fn schema(room: &Room, replay: Replay, me: Option<Kingdoms>) -> ElementBuilder {
    let mut attackers: Vec<Kingdoms> = Vec::new();
    for a in room.battles.iter().flat_map(|b| b.armies()) {
        if !attackers.contains(&a.attacker) {
            attackers.push(a.attacker);
        }
    }
    attackers.sort_by_key(|&a| Some(a) != me);
    let n = room.battles.len();
    let told = replay.current.min(n);
    let plural = |n: usize, s: &str| format!("{n} {s}{}", if n > 1 { "s" } else { "" });
    let count = if replay.done() {
        "tous tranchés".to_string()
    } else {
        format!("{told}/{n} tranché{}", if told > 1 { "s" } else { "" })
    };
    let expeditions: usize = room.battles.iter().map(|b| b.armies().len()).sum();
    let title = el(El::Div)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsBaseline,
            St::GapSm,
        ])
        .append([
            Text::new()
                .variant(TextVariant::Heading3)
                .content("La Campagne")
                .build(),
            el(El::Span)
                .st([St::TextXs, St::TextMuted, St::TabularNums, St::TextRight])
                .text(&format!(
                    "An {} · {} · {} · {count}",
                    room.game.year,
                    plural(expeditions, "expédition"),
                    plural(n, "front"),
                )),
        ]);
    let state = if replay.done() {
        "l'an s'achève"
    } else {
        "les armées se mettent en marche"
    };
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapMd])
        .append([
            title,
            el(El::Div).append([
                el(El::Div)
                    .st([St::DisplayFlex, St::JustifyBetween, St::ItemsBaseline])
                    .append([
                        eyebrow("Ordre de bataille"),
                        el(El::Span).st([St::TextXs, St::TextMuted]).text(state),
                    ]),
                order_of_battle(room, replay, me, &attackers),
            ]),
            section(
                "Fronts",
                el(El::Div)
                    .st([St::DisplayFlex, St::FlexCol, St::GapXs])
                    .append((0..n).map(|i| front_line(room, replay, i))),
            ),
        ])
}

/// The hero: attackers on the left, fronts on the right, each expedition a
/// line between them — marching for the front about to be fought, faint
/// once told.
fn order_of_battle(
    room: &Room,
    replay: Replay,
    me: Option<Kingdoms>,
    attackers: &[Kingdoms],
) -> ElementBuilder {
    let n = room.battles.len();
    let rows = attackers.len().max(n);
    let next = (!replay.done()).then_some(replay.current);
    // Before a front is fought, everything that is not on it fades away.
    let name_row = |p: Party, i: usize, right: bool, dim: bool| {
        el(El::Div)
            .st([
                St::H4_5rem,
                St::DisplayFlex,
                St::FlexCol,
                if right { St::ItemsEnd } else { St::ItemsStart },
                St::TextXs,
                St::LeadingTight,
                St::TextMuted,
                St::TabularNums,
                if right { St::TextRight } else { St::TextLeft },
                if dim { St::AnimateDim } else { St::Opacity100 },
            ])
            .style(
                Style::new()
                    .set("--kc", party_color(p))
                    .set("--d", &format!("{:.2}s", i as f32 * 0.09)),
            )
    };
    let big_name = |p: Party, right: bool, dead: bool, faint: bool, blink: bool| {
        el(El::Div)
            .st([
                St::TextXl,
                St::FontExtrabold,
                St::TextUppercase,
                St::LeadingNone,
                St::TrackingTight,
                St::TextParty,
                St::WhitespaceNowrap,
                if blink {
                    St::Blink
                } else if right {
                    St::AnimateInRight
                } else {
                    St::AnimateInLeft
                },
                if dead {
                    St::LineThrough
                } else if faint {
                    St::Opacity50
                } else {
                    St::Opacity100
                },
            ])
            .sm([St::Text2xl])
            .text(party_name(p))
    };
    let you = |p: Party| (p == me && me.is_some()).then(you_tag);
    let verdict = |text: String, bad: bool| {
        el(El::Span)
            .st([
                St::FontMono,
                St::TrackingWide,
                St::AnimateFadeUp,
                if bad { St::TextError } else { St::TextWarning },
            ])
            .text(&text)
    };

    let left = attackers.iter().enumerate().map(|(i, &a)| {
        let mine: Vec<(usize, &Army)> = room
            .battles
            .iter()
            .enumerate()
            .flat_map(|(i, b)| b.armies().iter().map(move |a| (i, a)))
            .filter(|(_, army)| army.attacker == a)
            .collect();
        let sent: i32 = mine.iter().map(|(_, a)| a.sent).sum();
        let known = mine.iter().all(|(i, _)| on_field(&room.battles[*i], me));
        let all_told = mine.iter().all(|(i, _)| replay.settled(*i));
        let won: i32 = mine
            .iter()
            .filter(|(_, a)| a.victory)
            .map(|(_, a)| a.spoils().arpents)
            .sum();
        let lost_all = all_told && won == 0;
        let dim = next.is_some_and(|n| !mine.iter().any(|(i, _)| *i == n));
        let mut row = name_row(Some(a), i, false, dim).append([
            big_name(Some(a), false, false, lost_all, false),
            el(El::Span)
                .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
                .append(
                    [
                        Some(el(El::Span).text(&hommes_heard(sent, known))),
                        you(Some(a)),
                    ]
                    .into_iter()
                    .flatten(),
                ),
        ]);
        if all_told {
            row = row.append([if won > 0 {
                verdict(format!("+{} arpents", fmt(won)), false)
            } else {
                verdict("anéantie".to_string(), true)
            }]);
        }
        row
    });
    let right = room.battles.iter().enumerate().map(|(i, b)| {
        let r = &b.result;
        let target = b.target;
        let annexed = replay.settled(i).then(|| b.annexed_by()).flatten();
        let mut lines = vec![big_name(
            target,
            true,
            annexed.is_some(),
            false,
            next == Some(i),
        )];
        if target.is_some() {
            let garrison = if on_field(b, me) {
                fmt(r.garrison_start)
            } else {
                "?".to_string()
            };
            lines.push(
                el(El::Span)
                    .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
                    .append(
                        [
                            you(target),
                            Some(el(El::Span).text(&format!("{garrison} en garnison"))),
                        ]
                        .into_iter()
                        .flatten(),
                    ),
            );
        }
        // The barbarian lands are known to all.
        lines.push(el(El::Span).text(&arpents_told(
            r.armies.iter().map(|a| a.line.arpents).sum::<i32>(),
            target.is_none() || on_field(b, me),
        )));
        if let Some(by) = annexed {
            lines.push(verdict(
                match target {
                    Some(_) => format!("annexée par la {}", by.name()),
                    None => format!("prises par la {}", by.name()),
                },
                true,
            ));
        } else if target.is_some() && replay.settled(i) {
            // The defender's outcome, as public as the attackers' land.
            let taken = r.spoils().arpents;
            lines.push(if taken > 0 {
                verdict(format!("−{} arpents", fmt(taken)), true)
            } else {
                verdict("a tenu".to_string(), false)
            });
        }
        name_row(target, i, true, next.is_some_and(|n| n != i)).append(lines)
    });

    // One bezier per expedition, from its attacker's row to its front's.
    let y = |i: usize| i as f64 * OB_ROW + 1.7;
    let mut paths = Vec::new();
    let mut slot = 0;
    for (i, b) in room.battles.iter().enumerate() {
        for a in b.armies() {
            let from = attackers.iter().position(|&k| k == a.attacker).unwrap_or(0);
            let d = format!(
                "M35.5,{y1:.1} C50,{y1:.1} 50,{y2:.1} 64.5,{y2:.1}",
                y1 = y(from),
                y2 = y(i)
            );
            let color = party_color(Some(a.attacker));
            let line = |st: St| {
                el(El::Path)
                    .st([st])
                    .at_str(At::D, &d)
                    .at(At::Fill, Av::None)
                    .at_str(At::Stroke, color)
                    .at_str(At::StrokeWidth, "2.5")
                    .at(At::StrokeLinecap, Av::Round)
                    .at(At::VectorEffect, Av::NonScalingStroke)
            };
            paths.push(
                line(St::PathDraw)
                    .attr("opacity", ".3")
                    .style(Style::new().set("--d", &format!("{:.2}s", 0.4 + slot as f32 * 0.08))),
            );
            slot += 1;
            if next == Some(i) {
                paths.push(line(St::PathMarch));
            } else if replay.settled(i) {
                paths.push(line(if a.victory {
                    St::Opacity75
                } else {
                    St::PathDead
                }));
            }
        }
    }
    let svg = el(El::Svg)
        .st([
            St::PositionAbsolute,
            St::Inset0,
            St::WFull,
            St::HFull,
            St::OverflowVisible,
        ])
        .at_str(At::ViewBox, &format!("0 0 100 {:.0}", rows as f64 * OB_ROW))
        .at(At::PreserveAspectRatio, Av::None)
        .append(paths);
    let column = |rows| {
        el(El::Div)
            .st([
                St::PositionRelative,
                St::W34Pct,
                St::DisplayFlex,
                St::FlexCol,
            ])
            .append(rows)
    };
    el(El::Div)
        .st([St::PositionRelative, St::DisplayFlex, St::JustifyBetween])
        .append([
            svg,
            column(left.collect::<Vec<_>>()),
            column(right.collect()),
        ])
}

/// One line of the fronts list: who marches on whom, and how it went.
fn front_line(room: &Room, replay: Replay, i: usize) -> ElementBuilder {
    let b = &room.battles[i];
    let r = &b.result;
    let names: Vec<&str> = r.armies.iter().map(|a| a.attacker.name()).collect();
    let (text, tone) = if replay.settled(i) {
        match b.annexed_by() {
            Some(by) => (format!("annexion par la {}", by.name()), St::TextWarning),
            None if r.garrison_fell() => (
                format!("victoire · +{} arpents", fmt(r.spoils().arpents)),
                St::TextWarning,
            ),
            None => (
                if r.armies.len() > 1 {
                    "anéanties".to_string()
                } else {
                    "anéantie".to_string()
                },
                St::TextError,
            ),
        }
    } else if !replay.done() && i == replay.current {
        ("à l'instant".to_string(), St::TextDefault)
    } else {
        ("à venir".to_string(), St::TextMuted)
    };
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsBaseline,
            St::GapSm,
            St::TextXs,
        ])
        .append([
            el(El::Span).st([St::TextMuted, St::Truncate]).append([
                mono(&format!("{} ", i + 1)),
                txt(&format!("{} → {}", names.join(", "), party_name(b.target))),
            ]),
            el(El::Span)
                .st([St::FontMono, St::TabularNums, St::WhitespaceNowrap, tone])
                .text(&text),
        ])
}

/// The little gold "vous" beside the viewer's own name.
fn you_tag() -> ElementBuilder {
    el(El::Span)
        .st([
            St::TextXs,
            St::FontMono,
            St::FontSemibold,
            St::TextUppercase,
            St::TrackingWidest,
            St::RoundedSm,
            St::PxXs,
            St::BgWarning,
            St::TextBlack,
            St::LeadingTight,
        ])
        .text("vous")
}

// ---------------------------------------------------------------------------
// B · the front
// ---------------------------------------------------------------------------

/// "FRONT 1/3 · Bretagne → Castille".
fn front_header(room: &Room, replay: Replay) -> ElementBuilder {
    let b = &room.battles[replay.current];
    let names: Vec<&str> = b.armies().iter().map(|a| a.attacker.name()).collect();
    el(El::Div)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsBaseline,
            St::GapSm,
            St::TextXs,
            St::TextMuted,
        ])
        .append([
            el(El::Span)
                .st([
                    St::FontMono,
                    St::TextUppercase,
                    St::TrackingWider,
                    St::TextDefault,
                ])
                .text(&format!(
                    "Front {}/{}",
                    replay.current + 1,
                    room.battles.len()
                )),
            el(El::Span).st([St::Truncate]).text(&format!(
                "{} → {}",
                names.join(", "),
                party_name(b.target)
            )),
        ])
}

/// A draining gauge: a name, "now / start", a bar in the party's colour.
struct Gauge<'a> {
    party: Party,
    name: &'a str,
    /// "garnison", "serfs"… under the name.
    kind: &'a str,
    now: i32,
    start: i32,
    /// The defender's side reads right-to-left.
    right: bool,
    /// The viewer stood on this front; else the figures read "?".
    known: bool,
}

impl Gauge<'_> {
    fn build(self, me: Option<Kingdoms>) -> ElementBuilder {
        let Gauge {
            party,
            name,
            kind,
            now,
            start,
            right,
            known,
        } = self;
        let dead = now <= 0;
        let share = pct(now as i64, start as i64);
        let figures = if known {
            format!("{} / {}", fmt(now.max(0)), fmt(start))
        } else {
            "? / ?".to_string()
        };
        el(El::Div)
            .st([St::MinW0])
            .style(Style::new().set("--kc", party_color(party)))
            .append([
                el(El::Div)
                    .st([
                        St::DisplayFlex,
                        St::JustifyBetween,
                        St::ItemsBaseline,
                        St::GapXs,
                        St::TextSm,
                        if right {
                            St::FlexRowReverse
                        } else {
                            St::FlexRow
                        },
                    ])
                    .append([
                        el(El::Span)
                            .st([St::DisplayFlex, St::ItemsCenter, St::GapXs, St::MinW0])
                            .append(
                                [
                                    Some(
                                        el(El::Span)
                                            .st([
                                                St::FontSemibold,
                                                St::Truncate,
                                                St::TextParty,
                                                if dead { St::Opacity50 } else { St::Opacity100 },
                                            ])
                                            .text(name),
                                    ),
                                    (party == me && me.is_some()).then(you_tag),
                                ]
                                .into_iter()
                                .flatten(),
                            ),
                        mono(&format!(
                            "{figures}{}",
                            if dead && start > 0 { " ✗" } else { "" }
                        )),
                    ]),
                el(El::Div)
                    .st([
                        St::TextXs,
                        St::TextMuted,
                        St::TextUppercase,
                        St::TrackingWider,
                        St::Opacity75,
                        if right { St::TextRight } else { St::TextLeft },
                    ])
                    .text(kind),
                el(El::Div)
                    .st([
                        St::PositionRelative,
                        St::H025rem,
                        St::RoundedFull,
                        St::BgMuted,
                        St::OverflowHidden,
                        St::MtXs,
                    ])
                    .append([el(El::Span)
                        .st([
                            if right { St::FillLeft } else { St::FillRight },
                            St::BgParty,
                            St::TransitionWidth,
                        ])
                        .style(Style::new().set("width", &share))]),
            ])
    }
}

fn battle(room: &Room, replay: Replay, me: Option<Kingdoms>) -> ElementBuilder {
    let b = &room.battles[replay.current];
    let r = &b.result;
    let round = replay.frame(b);
    let target = b.target;
    let known = on_field(b, me);
    let attackers = r.armies.iter().zip(&round.armies).map(|(a, s)| {
        Gauge {
            party: Some(a.attacker),
            name: a.attacker.name(),
            kind: "",
            now: s.men,
            start: a.sent,
            right: false,
            known,
        }
        .build(me)
    });
    // An empty garrison is a secret too: the others see a garrison.
    let garrison_kind = match (target, r.garrison_start, round.garrison) {
        (None, _, _) => "bande",
        (Some(_), 0, _) if known => "sans garnison",
        (Some(_), _, 0) => "garnison balayée",
        _ => "garnison",
    };
    let pool = vec![Gauge {
        party: target,
        name: party_name(target),
        kind: garrison_kind,
        now: round.garrison,
        start: r.garrison_start,
        right: true,
        known,
    }
    .build(me)];
    let column = |g: Vec<ElementBuilder>| {
        el(El::Div)
            .st([St::DisplayFlex, St::FlexCol, St::GapSm, St::MinW0])
            .append(g)
    };
    let mut body = vec![
        front_header(room, replay),
        el(El::Div)
            .st([
                St::DisplayGrid,
                St::GridColsFrAutoFr,
                St::ItemsStart,
                St::GapSm,
            ])
            .append([
                column(attackers.collect()),
                el(El::Span)
                    .st([
                        St::TextCenter,
                        St::TextSm,
                        St::LeadingNone,
                        St::PxXs,
                        St::PtMd,
                        St::TextError,
                        St::Blink,
                    ])
                    .text("⚔"),
                column(pool),
            ]),
    ];
    if target.is_some() && r.garrison_start > 0 && round.garrison == 0 {
        body.push(
            el(El::Div)
                .st([
                    St::TextCenter,
                    St::TextXs,
                    St::PySm,
                    St::RoundedMd,
                    St::BgSuccessSubtle,
                    St::TextOnSuccessSubtle,
                    St::FontSemibold,
                    St::AnimateFadeUp,
                ])
                .text("La garnison est balayée — victoire acquise"),
        );
    }
    body.push(march(room, b, round, known));
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapMd])
        .append(body)
}

/// The realm under attack: each army's march along its share of it, with
/// what it takes as it goes.
fn march(room: &Room, b: &Fought, round: &Round, known: bool) -> ElementBuilder {
    let r = &b.result;
    let target = b.target;
    // The barbarian lands are known to all.
    let known = known || target.is_none();
    let taken: i32 = round.armies.iter().map(|s| s.advance).sum();
    let total = format!(
        "{} · ",
        arpents_told(r.armies.iter().map(|a| a.line.arpents).sum::<i32>(), known)
    );
    let header = el(El::Div)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsBaseline,
            St::GapSm,
        ])
        .style(Style::new().set("--kc", party_color(target)))
        .append([
            el(El::Span)
                .st([St::FontSemibold, St::TextParty])
                .text(&if target.is_some() {
                    cap(&party_the(target))
                } else {
                    "Terres barbares".to_string()
                }),
            mono(&format!("{total}{} pris", fmt(taken))),
        ]);
    let n = r.armies.len();
    let armies = r
        .armies
        .iter()
        .zip(&round.armies)
        .map(|(a, s)| army_march(room, a, s, n, known));
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapSm])
        .append([header])
        .append(armies)
}

/// One army's march: its share of the realm as a bar filling with the
/// arpents taken, and under it what it took — goods, people, buildings, for
/// the viewer's own fronts.
fn army_march(room: &Room, a: &Army, s: &Stand, n: usize, known: bool) -> ElementBuilder {
    let len = a.line.arpents.max(1) as i64;
    let x = s.advance.clamp(0, a.line.arpents) as i64;
    let line = arpents_told(a.line.arpents, known);
    let share = match n {
        1 => format!("{line} · "),
        2 => format!("½ · {line} · "),
        3 => format!("⅓ · {line} · "),
        4 => format!("¼ · {line} · "),
        n => format!("1/{n} · {line} · "),
    };
    let header = el(El::Div)
        .st([
            St::DisplayFlex,
            St::JustifyBetween,
            St::ItemsBaseline,
            St::GapSm,
            St::TextXs,
        ])
        .append([
            el(El::Span)
                .st([St::FontSemibold, St::TextParty])
                .text(a.attacker.name()),
            el(El::Span)
                .st([St::FontMono, St::TextMuted, St::TabularNums])
                .append([
                    txt(&share),
                    el(El::Span)
                        .st([St::TextParty])
                        .text(&format!("{} pris", fmt(x as i32))),
                ]),
        ]);
    let bar = el(El::Div)
        .st([
            St::PositionRelative,
            St::H05rem,
            St::BgMuted,
            St::RoundedSm,
            St::OverflowHidden,
        ])
        .append([el(El::Span)
            .st([St::FillLeft, St::BgParty, St::TransitionWidth])
            .style(Style::new().set("width", &pct(x, len)))]);
    let spoils = Spoils {
        rallied: s.rallied,
        killed: s.killed,
        ..a.line.ground(s.advance)
    };
    let cur = room.game.kingdom(a.attacker).currency();
    let goods: Vec<String> = goods_fr(&spoils, cur)
        .into_iter()
        .map(|g| format!("+{g}"))
        .collect();
    let lines = [
        (goods, St::TextDefault),
        (people_fr(&spoils, Side::Attacker), St::TextDefault),
        (buildings_fr(&spoils), St::TextMuted),
    ]
    .into_iter()
    .filter(|(v, _)| known && !v.is_empty())
    .map(|(v, tone)| {
        el(El::Div)
            .st([St::TextXs, St::TabularNums, tone])
            .text(&v.join(" · "))
    });
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapXs])
        .style(Style::new().set("--kc", party_color(Some(a.attacker))))
        .append([header, bar])
        .append(lines)
}

// ---------------------------------------------------------------------------
// C · the verdict
// ---------------------------------------------------------------------------

fn verdict(room: &Room, replay: Replay, me: Option<Kingdoms>) -> ElementBuilder {
    let b = &room.battles[replay.current];
    let r = &b.result;
    let target = b.target;
    let known = on_field(b, me);
    let annexed = b.annexed_by();
    let (big, sub, tone) = match (annexed, r.garrison_fell()) {
        (Some(_), _) => (
            if target.is_some() {
                "Annexion"
            } else {
                "Conquête"
            },
            match target {
                Some(_) => format!("{} n'est plus", party_the(target)),
                None => "les dernières terres barbares sont prises".to_string(),
            },
            St::TextWarning,
        ),
        (None, true) => (
            "Victoire",
            match target {
                None => "la bande est défaite",
                Some(_) if known && r.garrison_start == 0 => "les serfs ont cédé",
                Some(_) => "la garnison est tombée",
            }
            .to_string(),
            St::TextSuccess,
        ),
        (None, false) => (
            if r.armies.len() > 1 {
                "Anéanties"
            } else {
                "Anéantie"
            },
            if target.is_none() {
                "la bande a tenu".to_string()
            } else {
                "la garnison a tenu".to_string()
            },
            St::TextError,
        ),
    };
    let mut body = vec![
        front_header(room, replay),
        el(El::Div)
            .st([
                St::DisplayFlex,
                St::ItemsBaseline,
                St::GapSm,
                St::AnimateFadeUp,
            ])
            .append([
                el(El::Span)
                    .st([
                        St::Text2xl,
                        St::FontExtrabold,
                        St::TextUppercase,
                        St::TrackingTight,
                        St::LeadingNone,
                        tone,
                    ])
                    .text(big),
                el(El::Span).st([St::TextSm, St::TextMuted]).text(&sub),
            ]),
    ];
    for (slot, a) in r.armies.iter().enumerate() {
        body.push(army_verdict(
            room,
            me,
            a,
            annexed == Some(a.attacker),
            slot,
            known,
        ));
    }
    body.push(defender_verdict(room, b, known));
    el(El::Div)
        .st([St::DisplayFlex, St::FlexCol, St::GapMd])
        .append(body)
}

/// One army's fate and its loot; off the viewer's fronts, the land alone.
fn army_verdict(
    room: &Room,
    me: Option<Kingdoms>,
    a: &Army,
    conqueror: bool,
    slot: usize,
    known: bool,
) -> ElementBuilder {
    let k = room.game.kingdom(a.attacker);
    let fate = if !a.victory {
        "pas un arpent"
    } else if a.men == 0 {
        "l'armée y est restée"
    } else {
        "l'armée rentre"
    };
    let who = el(El::Div)
        .st([
            St::DisplayFlex,
            St::ItemsBaseline,
            St::GapSm,
            St::FlexWrap,
            St::TextSm,
        ])
        .append([
            el(El::Span)
                .st([St::DisplayFlex, St::ItemsCenter, St::GapXs])
                .append(
                    [
                        Some(
                            el(El::Span)
                                .st([St::FontSemibold, St::TextParty])
                                .text(a.attacker.name()),
                        ),
                        (me == Some(a.attacker)).then(you_tag),
                    ]
                    .into_iter()
                    .flatten(),
                ),
            mono(&if known {
                format!(
                    "{} partis · {} perdus · {fate}",
                    hommes(a.sent),
                    fmt(a.lost())
                )
            } else {
                fate.to_string()
            }),
        ]);
    let mut card = el(El::Div)
        .st([
            St::DisplayFlex,
            St::FlexCol,
            St::GapSm,
            St::AnimateFadeUp,
            delay(slot + 1),
        ])
        .style(Style::new().set("--kc", party_color(Some(a.attacker))))
        .append([who]);
    if a.victory {
        let s = a.spoils();
        let mut loot: Vec<(String, &str)> = Vec::new();
        if conqueror {
            loot.push(("le royaume entier".to_string(), "arpents, serfs, blé, or"));
        } else {
            loot.push((format!("+{}", fmt(s.arpents)), "arpents"));
        }
        if !conqueror && known {
            if s.rallied.peasants > 0 {
                loot.push((format!("+{}", fmt(s.rallied.peasants)), "serfs ralliés"));
            }
            if s.grain > 0 {
                loot.push((format!("+{}", fmt(s.grain)), "boisseaux"));
            }
            if s.treasury > 0 {
                loot.push((format!("+{}", fmt(s.treasury)), k.currency()));
            }
            if s.rallied.merchants > 0 {
                loot.push((format!("+{}", s.rallied.merchants), "marchands ralliés"));
            }
            if s.rallied.nobles > 0 {
                loot.push((format!("+{}", s.rallied.nobles), "nobles ralliés"));
            }
        }
        card = card.append([el(El::Div)
            .st([St::DisplayGrid, St::GridCols2, St::GapSm])
            .append(loot.into_iter().map(|(v, label)| {
                el(El::Div)
                    .st([St::BgSurface, St::RoundedMd, St::PxMd, St::PySm])
                    .append([
                        el(El::Div)
                            .st([
                                St::FontSemibold,
                                St::TextLg,
                                St::LeadingTight,
                                St::TextSuccess,
                                St::TabularNums,
                            ])
                            .text(&v),
                        el(El::Div).st([St::TextXs, St::TextMuted]).text(label),
                    ])
            }))]);
        let killed = Spoils {
            rallied: People::default(),
            ..s
        };
        let mut rest = people_fr(&killed, Side::Attacker);
        rest.extend(buildings_fr(&s));
        if known && !rest.is_empty() {
            card = card.append([el(El::Div)
                .st([St::TextXs, St::TextMuted, St::TabularNums])
                .text(&rest.join(" · "))]);
        }
    }
    card
}

/// The defender's losses, once at the foot; off the viewer's fronts, no
/// headcount and the land to the hundred.
fn defender_verdict(room: &Room, b: &Fought, known: bool) -> ElementBuilder {
    let r = &b.result;
    let mut parts: Vec<String> = Vec::new();
    match b.target {
        Some(t) => {
            let k = room.game.kingdom(t);
            if known {
                if r.garrison_start > 0 {
                    parts.push(format!("{} d'armes tombés", hommes(r.garrison_fallen())));
                }
                parts.extend(people_fr(&r.spoils(), Side::Defender));
                parts.extend(lost_fr(&r.spoils(), k.currency()));
            }
            if b.annexed_by().is_some() {
                parts.push(format!("{} est déchu", k.player_name));
                parts.push("ses serfs changent de maître".to_string());
            } else if r.garrison_fell() {
                // The map only takes the outcome once everyone has read it:
                // the ground taken is still on the defender's count here.
                let left = (k.surface - r.spoils().arpents).max(0);
                parts.push(format!("{} restants", arpents_told(left, known)));
            } else {
                if known {
                    parts.push(format!("{} en garnison", fmt(r.garrison_left)));
                }
                parts.push("la terre est intacte".to_string());
            }
        }
        None => {
            parts.push(format!("{} de la bande tombés", fmt(r.garrison_fallen())));
            // The map only takes the outcome once everyone has read it.
            let left = (room.game.barbarians_surface - r.spoils().arpents).max(0);
            parts.push(if left > 0 {
                format!("{} arpents barbares restants", fmt(left))
            } else {
                "les barbares ont fui".to_string()
            });
        }
    }
    el(El::Div)
        .st([
            St::BorderT,
            St::PtSm,
            St::TextXs,
            St::TextMuted,
            St::TabularNums,
            St::AnimateFadeUp,
            delay(r.armies.len() + 1),
        ])
        .style(Style::new().set("--kc", party_color(b.target)))
        .append([
            el(El::Span)
                .st([
                    St::FontSemibold,
                    St::TextUppercase,
                    St::TrackingWide,
                    St::TextParty,
                    St::PrSm,
                ])
                .text(party_name(b.target)),
            txt(&parts.join(" · ")),
        ])
}
