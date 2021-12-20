use std::collections::{HashMap, HashSet};

use crate::types::{Branch, Host, NomadRef, RemoteNomadRefSet, User};

/// A point in time view of refs we care about. [`Snapshot`] is only for local branches and refs
/// and thus is scoped under a specific [`Config::user`].
#[allow(clippy::manual_non_exhaustive)]
pub struct Snapshot<Ref> {
    /// The active branches in this clone that the user manipulates directly with `git branch` etc.
    pub local_branches: HashSet<Branch>,
    /// The refs that nomad manages to follow the local branches.
    pub nomad_refs: Vec<NomadRef<Ref>>,
    /// Force all callers to go through [`Snapshot::new`] which can validate invariants.
    _private: (),
}

/// Describes where a ref should be removed from.
#[derive(Debug, PartialEq, Eq)]
pub enum PruneFrom<Ref> {
    LocalOnly(NomadRef<Ref>),
    LocalAndRemote(NomadRef<Ref>),
}

impl<Ref> Snapshot<Ref> {
    /// Smart constructor that enforces the "scoped under a specific [`Config::user`]" invariant.
    pub fn new(
        user: &User,
        local_branches: HashSet<Branch>,
        nomad_refs: Vec<NomadRef<Ref>>,
    ) -> Self {
        for nomad_ref in &nomad_refs {
            assert_eq!(user, &nomad_ref.user);
        }

        Snapshot {
            local_branches,
            nomad_refs,
            _private: (),
        }
    }

    /// Find nomad host branches that can be pruned because:
    /// 1. The local branch they were based on no longer exists.
    /// 2. The remote branch they were based on no longer exists.
    pub fn prune_deleted_branches(
        self,
        host: &Host,
        remote_nomad_refs: &RemoteNomadRefSet,
    ) -> Vec<PruneFrom<Ref>> {
        let Self {
            nomad_refs,
            local_branches,
            ..
        } = self;

        let mut prune = Vec::<PruneFrom<Ref>>::new();

        for nomad_ref in nomad_refs {
            if &nomad_ref.host == host {
                if !local_branches.contains(&nomad_ref.branch) {
                    prune.push(PruneFrom::LocalAndRemote(nomad_ref));
                }
            } else if !remote_nomad_refs.contains(&nomad_ref) {
                prune.push(PruneFrom::LocalOnly(nomad_ref));
            }
        }

        prune
    }

    /// Return all nomad branches regardless of host.
    pub fn prune_all(self) -> Vec<PruneFrom<Ref>> {
        let Self { nomad_refs, .. } = self;
        nomad_refs
            .into_iter()
            .map(PruneFrom::LocalAndRemote)
            .collect()
    }

    /// Return all nomad branches for specific hosts.
    pub fn prune_all_by_hosts(self, hosts: &HashSet<Host>) -> Vec<PruneFrom<Ref>> {
        let Self { nomad_refs, .. } = self;
        nomad_refs
            .into_iter()
            .filter_map(|nomad_ref| {
                if !hosts.contains(&nomad_ref.host) {
                    return None;
                }

                Some(PruneFrom::LocalAndRemote(nomad_ref))
            })
            .collect()
    }

    /// Return all [`NomadRef`]s grouped by host in sorted order.
    pub fn sorted_hosts_and_branches(self) -> Vec<(Host, Vec<NomadRef<Ref>>)> {
        let mut by_host = HashMap::<Host, Vec<NomadRef<Ref>>>::new();
        let Self { nomad_refs, .. } = self;

        for nomad_ref in nomad_refs {
            by_host
                .entry(nomad_ref.host.clone())
                .or_insert_with(Vec::new)
                .push(nomad_ref);
        }

        let mut as_vec = by_host
            .into_iter()
            .map(|(host, mut branches)| {
                branches.sort_by(|a, b| a.branch.cmp(&b.branch));
                (host, branches)
            })
            .collect::<Vec<_>>();
        as_vec.sort_by(|(host_a, _), (host_b, _)| host_a.cmp(host_b));

        as_vec
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, iter::FromIterator};

    use crate::types::{Host, RemoteNomadRefSet, User};

    use super::{Branch, NomadRef, PruneFrom, Snapshot};

    fn snapshot(
        user: &User,
        local_branches: impl IntoIterator<Item = &'static str>,
    ) -> Snapshot<()> {
        Snapshot::new(
            user,
            local_branches.into_iter().map(Branch::str).collect(),
            vec![
                NomadRef {
                    user: user.clone(),
                    host: Host::str("host0"),
                    branch: Branch::str("branch0"),
                    ref_: (),
                },
                NomadRef {
                    user: user.clone(),
                    host: Host::str("host0"),
                    branch: Branch::str("branch1"),
                    ref_: (),
                },
                NomadRef {
                    user: user.clone(),
                    host: Host::str("host1"),
                    branch: Branch::str("branch1"),
                    ref_: (),
                },
            ],
        )
    }

    fn remote_nomad_refs(
        collection: impl IntoIterator<Item = (&'static str, &'static str, &'static str)>,
    ) -> RemoteNomadRefSet {
        RemoteNomadRefSet::from_iter(
            collection.into_iter().map(|(user, host, branch)| {
                (User::str(user), Host::str(host), Branch::str(branch))
            }),
        )
    }

    /// Sets up the scenario where:
    ///
    ///     There are local branches
    ///     ... That DO NOT have nomad refs
    ///
    ///     There are local nomad refs from other hosts
    ///     ... That have corresponding remote nomad refs
    ///
    /// In this case, we should prune nothing.
    #[test]
    fn snapshot_prune_does_nothing0() {
        let prune = snapshot(&User::str("user0"), ["branch0", "branch1"]).prune_deleted_branches(
            &Host::str("host0"),
            &remote_nomad_refs([("user0", "host1", "branch1")]),
        );

        assert_eq!(prune, Vec::new());
    }

    /// Sets up the scenario where:
    ///
    ///     There are local branches
    ///     ... That have nomad refs
    ///
    ///     There are local nomad refs from other hosts
    ///     ... That have corresponding remote nomad refs
    ///
    /// In this case, we should prune nothing.
    #[test]
    fn snapshot_prune_does_nothing1() {
        let prune = snapshot(&User::str("user0"), ["branch0", "branch1"]).prune_deleted_branches(
            &Host::str("host0"),
            &remote_nomad_refs([
                ("user0", "host0", "branch0"),
                ("user0", "host0", "branch1"),
                ("user0", "host1", "branch1"),
            ]),
        );

        assert_eq!(prune, Vec::new());
    }

    /// Sets up the scenario where:
    ///
    ///     There are NO local branches
    ///     ... That have nomad refs
    ///
    ///     There are local nomad refs from other hosts
    ///     ... That have corresponding remote nomad refs
    ///
    /// In this case, we should remove the nomad refs for the local branches that no longer exist.
    #[test]
    fn snapshot_prune_removes_local_missing_branches() {
        let prune = snapshot(
            &User::str("user0"),
            [
                "branch0",
                // This branch has been removed
                // "branch1",
            ],
        )
        .prune_deleted_branches(
            &Host::str("host0"),
            &remote_nomad_refs([
                ("user0", "host0", "branch0"),
                ("user0", "host0", "branch1"),
                ("user0", "host1", "branch1"),
            ]),
        );

        assert_eq!(
            prune,
            vec![PruneFrom::LocalAndRemote(NomadRef {
                user: User::str("user0"),
                host: Host::str("host0"),
                branch: Branch::str("branch1"),
                ref_: (),
            })]
        );
    }

    /// Sets up the scenario where:
    ///
    ///     There are local branches
    ///     ... That have nomad refs
    ///
    ///     There are local nomad refs from other hosts
    ///     ... That DO NOT have corresponding remote nomad refs
    ///
    /// In this case, we should remove the local nomad refs from other hosts since the
    /// corresponding remote refs no longer exist.
    #[test]
    fn snapshot_prune_removes_remote_missing_branches() {
        let prune = snapshot(&User::str("user0"), ["branch0", "branch1"]).prune_deleted_branches(
            &Host::str("host0"),
            &remote_nomad_refs([
                ("user0", "host0", "branch0"),
                ("user0", "host0", "branch1"),
                // This remote nomad ref for another host has been removed
                // ("user0", "host1", "branch1"),
            ]),
        );

        assert_eq!(
            prune,
            vec![PruneFrom::LocalOnly(NomadRef {
                user: User::str("user0"),
                host: Host::str("host1"),
                branch: Branch::str("branch1"),
                ref_: (),
            })]
        );
    }

    /// [`Snapshot::prune_all`] should remove all branches.
    #[test]
    fn snapshot_prune_all() {
        let prune = snapshot(&User::str("user0"), ["branch0", "branch1"]).prune_all();
        assert_eq!(
            prune,
            vec![
                PruneFrom::LocalAndRemote(NomadRef {
                    user: User::str("user0"),
                    host: Host::str("host0"),
                    branch: Branch::str("branch0"),
                    ref_: (),
                }),
                PruneFrom::LocalAndRemote(NomadRef {
                    user: User::str("user0"),
                    host: Host::str("host0"),
                    branch: Branch::str("branch1"),
                    ref_: (),
                }),
                PruneFrom::LocalAndRemote(NomadRef {
                    user: User::str("user0"),
                    host: Host::str("host1"),
                    branch: Branch::str("branch1"),
                    ref_: (),
                }),
            ],
        );
    }

    /// [`Snapshot::prune_all_by_hosts`] should only remove branches for specified hosts.
    #[test]
    fn snapshot_prune_hosts() {
        let prune = snapshot(&User::str("user0"), ["branch0", "branch1"])
            .prune_all_by_hosts(&HashSet::from_iter([Host::str("host0")]));
        assert_eq!(
            prune,
            vec![
                PruneFrom::LocalAndRemote(NomadRef {
                    user: User::str("user0"),
                    host: Host::str("host0"),
                    branch: Branch::str("branch0"),
                    ref_: (),
                },),
                PruneFrom::LocalAndRemote(NomadRef {
                    user: User::str("user0"),
                    host: Host::str("host0"),
                    branch: Branch::str("branch1"),
                    ref_: (),
                },),
            ],
        );
    }
}