//! Empire — game rules for the classic *Empire.bas* kingdom simulation.
//!
//! All game data lives in [`EmpireGame`], a plain struct that derives
//! `rwire::State` so it can be used directly as handler state in an rwire app
//! (`fn handler(game: &mut EmpireGame)`) or owned by a synchronous front-end
//! such as the terminal `empire` app. The rule modules are pure functions over
//! `&EmpireGame` / `&mut EmpireGame` (or a single `&mut Kingdom`).

/// The rules' version: what a schooled brain was schooled under — the
/// game's rules, its sight and the reading of its answers. A school of
/// another version is not read, it is raised again.
pub const RULES: u32 = 2;

pub mod arena;
pub mod brain;
pub mod campaign;
pub mod demography;
pub mod economy;
pub mod events;
pub mod front;
pub mod game;
pub mod harvests;
pub mod intel;
pub mod investments;
pub mod kingdom;
pub mod random;
pub mod trade;
pub mod war;
pub mod weather;

pub use game::EmpireGame;
pub use kingdom::{Criterion, Fate, Kingdom, Kingdoms, PlayerTitle, Requirement, KINGDOMS};
pub use weather::Weather;
