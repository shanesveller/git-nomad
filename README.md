# git-nomad

Synchronize work-in-progress git branches in a light weight fashion. Motivation:

- You frequently work on the same repository from multiple machines, like a laptop and a desktop.
- You frequently rewrite history and use short understandable-by-only-you branch names, making pushing regular branches to git remotes cumbersome.
- You want all the efficiency and synchronization benefits of having a git clone available, meaning external syncing tools like Dropbox or network mounts are right out.
- You want this synchronization to work out-of-the-box with popular third party remote hosts like GitHub.

## Usage

[Install `git-nomad`](#installation) to make it available on your `$PATH`.

Configure it for a specific git clone:

```console
# Needs to be run once for each clone on each machine.
# Note that this defaults to your username and hostname.
# See `--help` for overriding this explicitly.
rraval@apollo:~/git-nomad$ git nomad init
Wrote Config {
    user: "rraval",
    host: "apollo",
}
```

Now hack away with your usual git workflow:

```console
rraval@apollo:~/git-nomad$ git checkout -b feature
rraval@apollo:~/git-nomad$ touch new_file
rraval@apollo:~/git-nomad$ git add .
rraval@apollo:~/git-nomad$ git commit -m "new file"
```

Whenever you like, you can push the state of your local branches with:

```console
# Synchronizes with a remote called `origin` by default.
# See `--help` for overriding this explicitly.
rraval@apollo:~/git-nomad$ git nomad sync
Pushing local branches to origin... 2s
Fetching branches from origin... 1s

apollo
  refs/nomad/apollo/master -> fe8bf41bbaf201c0506b60677f03a23da2873fdc
  refs/nomad/apollo/feature -> e02800d10b11ae03a93e43b8f7fc17b70dfe7acf
```

---

At some future point, you wish to pick up development on a different machine:

```console
# Only needs to be run once per clone on each machine.
rraval@boreas:~/git-nomad$ git nomad init
Wrote Config {
    user: "rraval",
    host: "boreas",
}
```

You can now run `sync` on this new machine as well:

```console
rraval@boreas:~/git-nomad$ git nomad sync
Pushing local branches to origin... 1s
Fetching branches from origin... 1s

apollo
  refs/nomad/apollo/master -> fe8bf41bbaf201c0506b60677f03a23da2873fdc
  refs/nomad/apollo/feature -> e02800d10b11ae03a93e43b8f7fc17b70dfe7acf
boreas
  refs/nomad/boreas/master -> fe8bf41bbaf201c0506b60677f03a23da2873fdc
```

Which prints out refs to use to pick up where you left off:

```console
rraval@boreas:~/git-nomad$ git checkout -b feature refs/nomad/apollo/feature
# Hack away
```

## How it works

Git is unabashedly a [content-addressed filesystem][git-cafs] that manipulates `blob`, `tree`, and `commit` objects. Layered on top of this is a half decent version control system, though this claim is contentious at best.

Git branches are implemented on top of [a more general scheme called `refs`][git-refs], where the local branch `master` is simply the commit pointed to by `refs/heads/master`. Git reserves a few hierarchies for its own use:

- `refs/heads/*` represent local branches.
- `refs/tags/*` represent tags.
- `refs/remotes/*` represent remote branches.

`git-nomad` works directly with refs to implement [its own light weight synchronization scheme][sync]:

1. Push local `refs/heads/*` to remote `refs/nomad/{user}/{host}/*`. This allows multiple users on multiple hosts to all use `git-nomad` on the same remote without overwriting data.
2. Fetch remote `refs/nomad/{user}/*` to local `refs/nomad/*`. This makes all the host refs for a given user available in a local clone.
3. Prune local `refs/nomad/*` refs where the corresponding branch has been deleted.

Using refs like this has advantages:

- You only pay the storage cost for the content unique to the branch. The bulk of repository history is shared!
- As refs get cleaned up, `git`s automatic garbage collection should reclaim space.
- Since these refs are under a separate `refs/nomad` hierarchy, they are not subject to the usual fast-forward only rules.

## Installation

FIXME: Publish to cargo.

FIXME: Make this runnable under nix.

[git-cafs]: https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
[git-refs]: https://git-scm.com/book/en/v2/Git-Internals-Git-References
[sync]: https://github.com/rraval/git-nomad/blob/master/src/command.rs