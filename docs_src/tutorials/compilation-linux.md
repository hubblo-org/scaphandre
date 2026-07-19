# Compile scaphandre from source (GNU/Linux)

We recommand using the latest version available of the rust toolchain, as the tests are calling for the latest version before running.

To be sure to be up to date, you may install rust from the [official website](https://www.rust-lang.org/) instead of your package manager.

To hack *scaph*, or simply be up to date with latest developments, you can download scaphandre from the main branch:

    git clone https://github.com/hubblo-org/scaphandre.git
    cd scaphandre
    cargo build # binary path is target/debug/scaphandre

To use the latest code for a true use case, build for release instead of debug:

    cargo build --release

Binary path is `target/release/scaphandre`.
