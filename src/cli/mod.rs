// Binary-only CLI modules, compiled into the `ani-cli-rs` binary (declared
// from `src/main.rs`), not into the `ani_lib` library.

mod schedule;
#[cfg(debug_assertions)]
mod showcase;

pub(crate) use schedule::display_next_episode_schedule;
#[cfg(debug_assertions)]
pub(crate) use showcase::{showcase_search, showcase_streams};
