// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// Runs inside game::integrated to observe its private live server token.
use super::{Flags, Game};
use crate::game::rusty_world_identity::SessionPathCache;
use std::{cell::RefCell, path::PathBuf};

thread_local! {
    static SOURCE: RefCell<SessionPathCache<Flags>> = RefCell::new(SessionPathCache::default());
}

/// Capture at tick_world_icon entry, before Game consumes the transient icon.
/// Integrated startup has already paired it with the live server token. This
/// is unconditional on the mod being enabled so enabling it later still works.
/// Same-token calls only compare Weak identities; no server IPC or disk access.
pub(in crate::game) fn capture(game: &Game<'_>) {
    if !game.native_window() {
        return;
    }
    let token = game.integrated.as_ref().map(|server| &server.flags);
    let candidate = game.world_icon.as_ref().and_then(|icon| icon.path.parent());
    SOURCE.with_borrow_mut(|source| source.capture(token, candidate));
}

/// Never authorize a path from the display name, "local" connection address,
/// an old server allocation or navigation's resettable dimension state.
pub(in crate::game) fn path(game: &Game<'_>) -> Option<PathBuf> {
    capture(game);
    if !game.native_window() {
        return None;
    }
    let token = game.integrated.as_ref().map(|server| &server.flags);
    SOURCE.with_borrow_mut(|source| source.path(token))
}
