//! Keeps steps that run at the same time from using the same things, see [`Resource`]

use crate::atoms::Resource;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Condvar, Mutex,
};

/// The resources of the steps that are running. A step waits until none of them conflicts with
/// its own
#[derive(Default)]
pub(super) struct Locks {
    /// The resources of each running step, by the number of its lock
    held: Mutex<Vec<(u64, Vec<Resource>)>>,
    /// Woken when a step lets go of its resources
    released: Condvar,
    /// The number of the last lock
    last: AtomicU64,
}

impl Locks {
    /// Waits until no running step uses anything that conflicts with `resources`, then holds
    /// them until the returned guard is dropped
    pub(super) fn lock(&self, resources: Vec<Resource>) -> Guard<'_> {
        let id = self.last.fetch_add(1, Ordering::Relaxed) + 1;
        let mut held = self
            .held
            .lock()
            .expect("no step panics while it holds the table");
        while conflicts(&held, &resources) {
            held = self
                .released
                .wait(held)
                .expect("no step panics while it holds the table");
        }
        held.push((id, resources));

        Guard { locks: self, id }
    }

    /// Lets go of the resources of lock `id`, and wakes the steps that wait
    fn release(&self, id: u64) {
        let mut held = self
            .held
            .lock()
            .expect("no step panics while it holds the table");
        held.retain(|(held_id, _)| *held_id != id);
        self.released.notify_all();
    }
}

/// Whether anything in `held` conflicts with `wanted`
fn conflicts(held: &[(u64, Vec<Resource>)], wanted: &[Resource]) -> bool {
    held.iter()
        .flat_map(|(_, resources)| resources)
        .any(|held| wanted.iter().any(|wanted| held.conflicts(wanted)))
}

/// Holds the resources of one step, see [`Locks::lock`]
pub(super) struct Guard<'a> {
    /// The table the resources are in
    locks: &'a Locks,
    /// The number of this lock
    id: u64,
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.locks.release(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    fn path(path: &str) -> Vec<Resource> {
        vec![Resource::Path(PathBuf::from(path))]
    }

    /// Runs `first` and `second` in two threads, each holding its lock for a moment, and
    /// returns how many held one at the same time at most
    fn most_at_once(first: Vec<Resource>, second: Vec<Resource>) -> usize {
        let locks = Locks::default();
        let holding = AtomicUsize::new(0);
        let most = AtomicUsize::new(0);

        std::thread::scope(|scope| {
            for resources in [first, second] {
                let (locks, holding, most) = (&locks, &holding, &most);
                scope.spawn(move || {
                    let _guard = locks.lock(resources);
                    let now = holding.fetch_add(1, Ordering::SeqCst) + 1;
                    most.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(100));
                    holding.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });

        most.into_inner()
    }

    #[test]
    fn conflicting_resources_take_turns() {
        assert_eq!(1, most_at_once(path("/etc"), path("/etc/hosts")));
        assert_eq!(1, most_at_once(vec![Resource::Anything], path("/tmp/a")));
    }

    #[test]
    fn other_resources_are_held_at_the_same_time() {
        assert_eq!(2, most_at_once(path("/tmp/a"), path("/tmp/b")));
    }
}
