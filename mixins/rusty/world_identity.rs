// Copyright 2026 Justeck. SPDX-License-Identifier: Apache-2.0
// One verified local save path tied to one live integrated-server allocation.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

struct Entry<T> {
    token: Weak<T>,
    path: PathBuf,
}

/// Stores at most one path and one weak allocation identity. Equal token
/// contents are insufficient: only the same live Arc allocation can use it.
/// Keeping the Weak also prevents allocation-address reuse while cached; it
/// does not keep the integrated server's Flags or world data alive.
pub struct SessionPathCache<T> {
    entry: Option<Entry<T>>,
}

impl<T> Default for SessionPathCache<T> {
    fn default() -> Self {
        Self { entry: None }
    }
}

impl<T> SessionPathCache<T> {
    /// A missing candidate for the same session means the transient icon was
    /// consumed. A missing/different session immediately invalidates the old
    /// path, including when the old server still has strong owners elsewhere.
    pub fn capture(&mut self, token: Option<&Arc<T>>, candidate: Option<&Path>) {
        let Some(token) = token else {
            self.entry = None;
            return;
        };
        let weak = Arc::downgrade(token);
        if self
            .entry
            .as_ref()
            .is_some_and(|entry| entry.token.ptr_eq(&weak))
        {
            return;
        }
        self.entry = candidate.map(|path| Entry {
            token: weak,
            path: path.to_owned(),
        });
    }

    /// Validate against the current session before returning an owned path.
    /// Navigation/dimension state can reset independently of this cache.
    pub fn path(&mut self, token: Option<&Arc<T>>) -> Option<PathBuf> {
        self.capture(token, None);
        self.entry.as_ref().map(|entry| entry.path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc};

    fn folder(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("rusty-world-identity-fixture")
            .join(name)
    }

    #[test]
    fn transient_icon_disappearance_preserves_the_live_session_path() {
        let token = Arc::new(());
        let path = folder("world-a");
        let mut cache = SessionPathCache::default();
        cache.capture(Some(&token), Some(&path));
        cache.capture(Some(&token), None);
        assert_eq!(cache.path(Some(&token)), Some(path));
    }

    #[test]
    fn different_sessions_cannot_reuse_a_display_name_or_previous_path() {
        // Equal values/display names do not establish allocation identity.
        let first = Arc::new("Same display name");
        let second = Arc::new("Same display name");
        let mut cache = SessionPathCache::default();
        cache.capture(Some(&first), Some(&folder("world-a")));
        assert_eq!(cache.path(Some(&second)), None);
        cache.capture(Some(&second), Some(&folder("world-b")));
        assert_eq!(cache.path(Some(&first)), None);
        assert_eq!(cache.path(Some(&second)), None);
    }

    #[test]
    fn dropping_the_server_does_not_retain_its_data_or_authorize_a_new_token() {
        let first = Arc::new(vec![42; 1024]);
        let weak = Arc::downgrade(&first);
        let mut cache = SessionPathCache::default();
        cache.capture(Some(&first), Some(&folder("world-a")));
        assert_eq!(Arc::strong_count(&first), 1);
        drop(first);
        assert!(weak.upgrade().is_none());
        let next = Arc::new(vec![42; 1024]);
        assert_eq!(cache.path(Some(&next)), None);
        cache.capture(Some(&next), Some(&folder("world-b")));
        assert_eq!(cache.path(Some(&next)), Some(folder("world-b")));
    }

    #[test]
    fn disconnect_clears_the_path_even_when_the_old_server_still_has_owners() {
        let token = Arc::new(());
        let mut cache = SessionPathCache::default();
        cache.capture(Some(&token), Some(&folder("world-a")));
        assert_eq!(cache.path(None), None);
        assert_eq!(cache.path(Some(&token)), None);
    }

    #[test]
    fn separate_scope_lifetimes_and_missing_icons_do_not_reset_session_identity() {
        let token = Arc::new(());
        let path = folder("world-a");
        let mut cache = SessionPathCache::default();
        cache.capture(Some(&token), Some(&path));
        // Dimension/navigation state lives outside this session-only cache.
        for dimension in ["overworld", "nether", "overworld"] {
            let navigation_scope = dimension.to_owned();
            assert_eq!(cache.path(Some(&token)), Some(path.clone()));
            drop(navigation_scope);
            cache.capture(Some(&token), None);
        }
        assert_eq!(cache.path(Some(&token)), Some(path));
    }

    #[test]
    fn replacement_is_bounded_to_one_weak_token_and_one_path() {
        let first = Arc::new(());
        let second = Arc::new(());
        let mut cache = SessionPathCache::default();
        cache.capture(Some(&first), Some(&folder("world-a")));
        assert_eq!(Arc::weak_count(&first), 1);
        cache.capture(Some(&second), Some(&folder("world-b")));
        assert_eq!(Arc::weak_count(&first), 0);
        assert_eq!(Arc::weak_count(&second), 1);
        assert_eq!(cache.path(Some(&second)), Some(folder("world-b")));
        cache.capture(None, None);
        assert_eq!(Arc::weak_count(&second), 0);
    }
}
