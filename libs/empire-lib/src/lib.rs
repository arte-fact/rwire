//! Empire — game rules for the classic *Empire.bas* kingdom simulation.
//!
//! All game data lives in [`EmpireGame`], a plain struct that derives
//! `rwire::State` so it can be used directly as handler state in an rwire app
//! (`fn handler(game: &mut EmpireGame)`) or owned by a synchronous front-end
//! such as the terminal `empire` app. The rule modules are pure functions over
//! `&EmpireGame` / `&mut EmpireGame` (or a single `&mut Kingdom`).

pub mod demography;
pub mod economy;
pub mod events;
pub mod game;
pub mod harvests;
pub mod ia;
pub mod investments;
pub mod kingdom;
pub mod random;
pub mod trade;
pub mod war;
pub mod weather;

pub use game::EmpireGame;
pub use kingdom::{Kingdom, Kingdoms, PlayerTitle, KINGDOMS};
pub use weather::Weather;
