# crystal-sphinx

Crystal Sphinx is a game written in Rust using TemportalEngine which is heavily inspired by Minecraft.
Its a voxel/block based game that enthusiastically supports multiplayer and creativity.
It diverges from the Minecraft experience, however, in that it is not a currated game with a specific set of rules / design expectations.
CS' ethusiastic support of creavitity extends to both enabling players in the core game systems,
as well as enabling the community to easily slot in their own modules/plugins/mods to change their experience.
Crystal Sphinx (and TemportalEngine) are both entirely open source, and as such are easily modifiable by the
community to further support the aforementioned module development.

TODO:
will need to separate to its own git repository at some point https://stosb.com/blog/retaining-history-when-moving-files-across-repositories-in-git/
this operation will need to retain move history (i.e. `git log --name-only --format=format: --follow -- path/to/file | sort -u`)

Library Notes:
- [libloading](https://docs.rs/libloading/0.7.0/libloading/) for plugin loading/execution. [See guide for more.](https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html)
- [async asset loading](https://rust-lang.github.io/async-book/01_getting_started/02_why_async.html)
- [networking - laminar](https://crates.io/crates/laminar) as a replacement for Game Networking Sockets
- [physics - rapier](https://crates.io/crates/rapier3d)
- [profiling](https://crates.io/crates/profiling)
- [cryptography](https://crates.io/crates/rustls)
- [noise](https://crates.io/crates/noise) for randomization and noise in chunk generation
- [specs](https://crates.io/crates/specs) [book](https://specs.amethyst.rs/docs/tutorials)
- [anymap](https://crates.io/crates/anymap)
