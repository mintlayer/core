# Mintlayer core contributing guide

We are happy to take contributions to the project in any form: if you find a bug feel free to create an issue, if you fix a bug feel free to create a pr to merge you fix, if you want to add a totally new feature go ahead and do so.


## Setup

Mintlayer is built using substrate which means you'll need rust and substrate set up to do any development work. The functional tests require python to be installed too.

You can follow these steps below or go substrate's own guide [here](https://docs.substrate.io/v3/getting-started/overview/)
### Build essentials

#### Ubuntu
    sudo apt update
    sudo apt install -y git clang curl libssl-dev llvm libudev-dev

#### Fedora
    sudo dnf update
    sudo dnf install clang curl git openssl-devel

#### Arch
    pacman -Syu --needed --noconfirm curl git clang
    
#### MacOS (Note there are issues with M1 processors - you're on your own for now :D)
    brew update
    brew install openssl
    

### Rust
    curl https://sh.rustup.rs -sSf | sh
    source ~/.cargo/env
    rustup default stable
    rustup update
    rustup update nightly
    rustup target add wasm32-unknown-unknown --toolchain nightly

### Functional test stuff
Ensure python3 is already installed. I'll be shocked if it isn't. After that install the dependencies listed below.

    python -m pip install 'substrate-interface == 0.13.12'
    python -m pip install 'scalecodec == 0.11.18'
    
## Building Mintlayer

To build Mintlayer go to [the main repo](https://github.com/mintlayer/core) and clone the repo using git like you would with any other repo.
Development happens on the staging branch so make sure you can build Mintlayer by checking out the staging branch and building the code base.

    git fetch --all
    git checkout staging
    cargo build
    
The above will build the code base in debug mode. If you go to `target/debug` you should find the `mintlayer-core` binary. If you have issues building it's probably 
the installation steps above, double check them and try again. If the build keeps failing then feel free to reach out to us in an issue or on [discord](https://discord.gg/XMrhvngQ8T).

If that all goes swimmingly then it's time to do something very very slightly more exciting. Make sure you can run the unit tests using `cargo test`
   
Yep wasn't that exciting. All of those tests should pass, if they don't feel free to dig in and find out what's going on.

To build a release version of Mintlayer you simply run `cargo build --release`
    
To run the functional tests, ensure Mintlayer has been compiled first, go to `test` and run `test_runner.py`. Everything else will happen automatically.

## How to actually contribute

The first thing to do, once you know what you want to do, is to open an issue. If you think you'd found a bug open an issue so it can be discussed with the wider
community. If you think you've got a snazzy new idea for a feature, open an issue and we'll discuss it as a community; maybe someone else is already working on it...

Whatever it is you're working on you'll want to create a branch for you bug fix or feature from staging
 
 
   git checkout staging
   git checkout -b my_new_branch
   
   
I'd suggest you pick a better name than that though, something which makes it obvious what you're working on is preferred. Once you're done the first step is to make
sure that the existing functional tests and unit tests still work. If you've broken something it's time to go and fix that first. Once the existing tests are good
it's time for you to add your own. Even for small bug fixes adding a unit test is worth the effort to ensure the bug isn't reintroduced later. For new features functional tests
are a hard requirement. Make life as easy as possible for the reviewer when they have to look at the actual code. What testing have you done? Have you run any benchmarks?

Once you've created a set of tests that prove out your code create a pr to merge your branch in to staging. Make sure you write a good pr. Explain what you're doing, 
explain why you're doing it, explain how this interacts with the existing code base and explain how it works. Make sure to link to the open issue too. When you pick
reviewers GitHub will likely recommend some people. If not tag anyone and they can help get the right people involved. Your code will either be merged or changes will be requested.
Before you open the PR, think about what else you can do to make the reviewers life easier… Can you run cargo-audit to find known issues in libraries? Could you run a fuzzer or static analyser? Have you checked the licenses of any libraries you’re using?
A good pull request should try and limit the number of lines changed in the request, as a general rule it takes roughly 60 mins to review a 300-400 line request so try and keep PRs to 300-400 lines in total.
A pull request should try and deal with a single issue be it a new feature or a bug fix. If you’re fixing 2 separate unrelated bugs then open 2 PRs. A PR should be made of logical commits, that could be a single commit for a simple bug or several commits for a more complex new feature. If you refactor some existing code and add a new feature in the same PR then that should be at least 2 commits.


## A quick guide to Mintlayer


    

