FROM debian:bullseye

ENV REQUIRED_PACKAGES git clang curl libssl-dev llvm libudev-dev

RUN apt-get update \
        && apt-get install -y $REQUIRED_PACKAGES \
        && curl https://sh.rustup.rs -sSf | sh -s -- -y \
        && $HOME/.cargo/bin/rustup default stable \
        && $HOME/.cargo/bin/rustup update \
        && $HOME/.cargo/bin/rustup update nightly \
        && $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown --toolchain nightly

RUN apt-get update \
        && apt-get install -y clang-9 \
        && git clone https://github.com/mintlayer/core \
        && cd core \
        && $HOME/.cargo/bin/cargo build --release

WORKDIR /core

EXPOSE 30333

CMD RUST_LOG=info target/release/mintlayer-core --base-path /tmp/ml-core --chain=assets/Testnet1Spec.json
