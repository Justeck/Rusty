// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// API359 adapter. All actions are local state, mod data files or clipboard.
use super::Game;
use horizon_client::{clipboard::Clipboard, hud::HudBatch};
use horizon_client_render::mesher::{rusty, rusty_navigation as nav};
use std::{cell::RefCell, path::PathBuf};

#[derive(Clone, PartialEq)]
enum IdentitySource {
    Local(PathBuf),
    Server(String),
}
#[derive(Default)]
struct State {
    data: nav::NavigationData,
    directory: Option<PathBuf>,
    load_error: Option<String>,
    source: Option<IdentitySource>,
    identity: Option<String>,
    scope: Option<nav::Scope>,
    trail: nav::BreadcrumbTrail,
    death: nav::DeathTracker,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

/// This does not use the display world name or the shared integrated address
/// "local". Capture the authoritative saves folder at integrated startup;
/// Horizon consumes world_icon after writing a screenshot, so that transient
/// field cannot itself identify the world during later navigation commands.
fn source(game: &Game<'_>) -> Result<IdentitySource, String> {
    let integrated_path = super::integrated::rusty_world_source::path(game);
    if game.integrated.is_some() {
        let folder = integrated_path.ok_or("Local world identity is not available yet")?;
        return Ok(IdentitySource::Local(folder));
    }
    if let Some(remote) = &game.remote {
        if remote.session.address == "local" {
            return Err("Local connection has no verified world identity".into());
        }
        return Ok(IdentitySource::Server(remote.session.address.clone()));
    }
    // Covers a host-only local Game if supported by the engine; the normal
    // integrated window takes the icon-folder path above.
    if !game.world_root.as_os_str().is_empty() {
        return Ok(IdentitySource::Local(game.world_root.clone()));
    }
    Err("No navigation world/server identity".into())
}
impl State {
    fn sync(&mut self, game: &Game<'_>) -> Result<(), String> {
        let player = game
            .native_player()
            .ok_or("Open a world before using waypoints")?;
        let directory = game.native_data_dir(rusty::ID);
        if self.directory.as_ref() != Some(&directory) {
            *self = Self::default();
            self.directory = Some(directory.clone());
            match nav::NavigationData::load(&directory) {
                Ok(data) => self.data = data,
                Err(error) => self.load_error = Some(error),
            }
        }
        let source = source(game)?;
        if self.source.as_ref() != Some(&source) {
            self.identity = None;
            self.scope = None;
            self.trail.reset();
            let identity = match &source {
                IdentitySource::Local(path) => {
                    let path = path
                        .canonicalize()
                        .map_err(|e| format!("Cannot identify local world folder: {e}"))?;
                    format!(
                        "local:{}",
                        path.to_str()
                            .ok_or("Local world path is not valid Unicode")?
                    )
                }
                IdentitySource::Server(address) => format!("server:{address}"),
            };
            self.source = Some(source);
            self.identity = Some(identity);
        }
        let scope = nav::Scope::new(
            self.identity
                .as_deref()
                .ok_or("Navigation identity unavailable")?,
            player.dimension,
        )?;
        if self.scope.as_ref() != Some(&scope) {
            self.trail.reset();
            self.scope = Some(scope);
        }
        if let Some(error) = &self.load_error {
            return Err(format!(
                "Navigation file was not loaded; preserving it: {error}"
            ));
        }
        Ok(())
    }
    fn update<T>(
        &mut self,
        game: &Game<'_>,
        action: impl FnOnce(&mut nav::NavigationData, &nav::Scope) -> Result<T, String>,
    ) -> Result<T, String> {
        self.sync(game)?;
        let scope = self.scope.as_ref().ok_or("No navigation scope")?;
        let mut next = self.data.clone();
        let result = action(&mut next, scope)?;
        next.save(
            self.directory
                .as_deref()
                .ok_or("No navigation data folder")?,
        )?;
        self.data = next;
        Ok(result)
    }
}

/// Call from the client native_frame after config load, not on server Games.
/// No persistence or scanning is performed merely to show a compass.
#[allow(dead_code)]
pub(super) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    if !game.native_on(rusty::ID) || game.native_player().is_none() || game.title.is_some() {
        reset();
        return;
    }
    let cfg = rusty::config();
    if !cfg.navigation.breadcrumbs_enabled {
        STATE.with_borrow_mut(|state| state.trail.reset());
    }
    if !cfg.navigation.waypoint_hud
        && !cfg.navigation.breadcrumbs_enabled
        && !cfg.navigation.death_markers
    {
        STATE.with_borrow_mut(|state| {
            state.death.observe(false, None, false);
        });
        return;
    }
    STATE.with_borrow_mut(|state| {
        if state.sync(game).is_err() {
            state.scope = None;
            state.trail.reset();
            return;
        }
        let Some(player) = game.native_player() else {
            return;
        };
        let Some(scope) = state.scope.as_ref() else {
            return;
        };
        let alive = game
            .player
            .as_ref()
            .is_some_and(|player| !player.health.dead_or_dying());
        if state
            .death
            .observe(cfg.navigation.death_markers, Some(scope), alive)
        {
            let name = format!(
                "Death {}",
                game.native_clock().map_or(0, |clock| clock.game_time)
            );
            let position = [player.x, player.y, player.z];
            // Keep the existing selected marker. A death never starts navigation.
            match state.update(game, |data, scope| data.add(scope, &name, position)) {
                Ok(_) => {
                    game.native_chat("Rusty saved a death marker in this world and dimension.")
                }
                Err(error) => {
                    game.native_chat(&format!("Death marker could not be saved: {error}"))
                }
            }
        }
        let Some(scope) = state.scope.as_ref() else {
            return;
        };
        let loaded = game
            .world
            .loaded(player.x.floor() as i32, player.z.floor() as i32);
        if state
            .trail
            .observe(
                scope,
                [player.x, player.y, player.z],
                loaded,
                &cfg.navigation,
            )
            .is_err()
        {
            state.trail.reset();
        }
    });
}

/// Call from native_hud; ordinary HUD visibility and Minecraft's font apply.
#[allow(dead_code)]
pub(super) fn hud(game: &Game<'_>, batch: &mut HudBatch) {
    if !game.native_window() || !game.native_on(rusty::ID) || game.ui.hide_gui {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };
    let cfg = rusty::config();
    let settings = &cfg.navigation;
    if !settings.compass_enabled && !settings.waypoint_hud {
        return;
    }
    let mut lines = Vec::new();
    if settings.compass_enabled
        && let Ok(heading) = nav::compass(f64::from(player.yaw))
    {
        lines.push(format!(
            "{heading}  {:.0}°",
            f64::from(player.yaw).rem_euclid(360.)
        ));
    }
    let compass_lines: Vec<_> = lines.drain(..).map(|line| (line, 0xffedddca)).collect();
    super::rusty_hud_native::lines(
        game,
        batch,
        &cfg.client.hud_layout,
        "compass",
        &compass_lines,
    );
    if settings.waypoint_hud {
        STATE.with_borrow(|state| {
            // A frame sync owns scope discovery; never show data from a stale
            // server/dimension if HUD is requested between transition hooks.
            let current = source(game).ok();
            if current.as_ref() != state.source.as_ref()
                || state
                    .scope
                    .as_ref()
                    .is_none_or(|s| s.dimension != player.dimension)
            {
                return;
            }
            if let Some(scope) = &state.scope
                && let Some(waypoint) = state.data.selected(scope)
                && let Ok(to) = nav::direction(
                    [player.x, player.y, player.z],
                    waypoint.position,
                    f64::from(player.yaw),
                )
            {
                let heading = nav::compass(to.bearing).unwrap_or("?");
                lines.push(format!(
                    "{}  {:.0}m {heading} ({:+.0}°)",
                    waypoint.name, to.distance, to.relative
                ));
            }
        });
    }
    let waypoint_lines: Vec<_> = lines.into_iter().map(|line| (line, 0xffedddca)).collect();
    super::rusty_hud_native::lines(
        game,
        batch,
        &cfg.client.hud_layout,
        "waypoints",
        &waypoint_lines,
    );
}

/// Reset on clear_entities/disconnect/dimension transition. The durable data
/// remains on disk; the next used frame reloads it in the right scope.
#[allow(dead_code)]
pub(super) fn reset() {
    STATE.with_borrow_mut(|state| *state = State::default());
}
/// Call at the client `remote_teleport` hook, including same-dimension short
/// teleports/corrections. A distance threshold alone cannot identify those.
#[allow(dead_code)]
pub(super) fn teleported() {
    STATE.with_borrow_mut(|state| state.trail.reset());
}
fn check(game: &Game<'_>) -> Result<(), String> {
    if !game.native_window() || !game.native_on(rusty::ID) {
        Err("Rusty navigation is disabled".into())
    } else {
        Ok(())
    }
}
#[allow(dead_code)]
pub(super) fn add_current(game: &mut Game<'_>, name: &str) -> Result<u64, String> {
    check(game)?;
    let player = game
        .native_player()
        .ok_or("Open a world before adding a waypoint")?;
    STATE.with_borrow_mut(|state| {
        state.update(game, |data, scope| {
            let id = data.add(scope, name, [player.x, player.y, player.z])?;
            data.select(scope, Some(id))?;
            Ok(id)
        })
    })
}
#[allow(dead_code)]
pub(super) fn select(game: &mut Game<'_>, id: Option<u64>) -> Result<(), String> {
    check(game)?;
    STATE.with_borrow_mut(|state| state.update(game, |data, scope| data.select(scope, id)))
}
#[allow(dead_code)]
pub(super) fn remove(game: &mut Game<'_>, id: u64) -> Result<(), String> {
    check(game)?;
    STATE.with_borrow_mut(|state| state.update(game, |data, scope| data.remove(scope, id)))
}
#[allow(dead_code)]
pub(super) fn copy(game: &mut Game<'_>, id: u64) -> Result<(), String> {
    check(game)?;
    let text = STATE.with_borrow_mut(|state| {
        state.sync(game)?;
        state
            .data
            .coordinates(state.scope.as_ref().ok_or("No navigation scope")?, id)
    })?;
    game.clipboard.set(&text);
    Ok(())
}
#[allow(dead_code)]
pub(super) fn waypoints(game: &Game<'_>) -> Result<Vec<nav::Waypoint>, String> {
    check(game)?;
    STATE.with_borrow_mut(|state| {
        state.sync(game)?;
        Ok(state
            .data
            .waypoints(state.scope.as_ref().ok_or("No navigation scope")?)
            .to_vec())
    })
}
/// A selected marker paired with its verified world/server and dimension scope.
#[allow(dead_code)]
pub(super) fn selected(game: &Game<'_>) -> Result<Option<(nav::Scope, nav::Waypoint)>, String> {
    check(game)?;
    STATE.with_borrow_mut(|state| {
        state.sync(game)?;
        let scope = state.scope.as_ref().ok_or("No navigation scope")?;
        Ok(state
            .data
            .selected(scope)
            .cloned()
            .map(|point| (scope.clone(), point)))
    })
}
/// Optional gizmo integration. Points are observed body locations only; the
/// caller should render these in the ordinary depth-tested line pass.
#[allow(dead_code)]
pub(super) fn segments(game: &Game<'_>) -> Vec<[[f64; 3]; 2]> {
    if !game.native_window()
        || !game.native_on(rusty::ID)
        || !rusty::config().navigation.breadcrumbs_enabled
    {
        return Vec::new();
    }
    STATE.with_borrow(|state| {
        let Some(player) = game.native_player() else {
            return Vec::new();
        };
        if source(game).ok().as_ref() != state.source.as_ref()
            || state
                .scope
                .as_ref()
                .is_none_or(|scope| scope.dimension != player.dimension)
        {
            return Vec::new();
        }
        state
            .trail
            .segments()
            .into_iter()
            .filter(|[a, b]| {
                // Conservative chunk rectangle, bounded by teleport_distance.
                // Hide any segment
                // whose path may cross a column the client no longer has.
                let min_x = (a[0].min(b[0]).floor() as i32) >> 4;
                let max_x = (a[0].max(b[0]).floor() as i32) >> 4;
                let min_z = (a[2].min(b[2]).floor() as i32) >> 4;
                let max_z = (a[2].max(b[2]).floor() as i32) >> 4;
                (min_x..=max_x).all(|x| (min_z..=max_z).all(|z| game.world.loaded(x * 16, z * 16)))
            })
            .collect()
    })
}

/// Verified source identity and received dimension, shared by scoped tools.
pub(super) fn scope(game: &Game<'_>) -> Result<nav::Scope, String> {
    STATE.with_borrow_mut(|state| {
        state.sync(game)?;
        state
            .scope
            .clone()
            .ok_or_else(|| "World identity unavailable".into())
    })
}
