set windows-shell := ["pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @vx just --list

fmt:
    vx cargo fmt --all

fmt-check:
    vx cargo fmt --all -- --check

check:
    vx cargo check --workspace

lint:
    vx cargo clippy --workspace --all-targets -- -D warnings

test:
    vx cargo test --workspace

coverage:
    vx cargo llvm-cov --workspace --lcov --output-path lcov.info

coverage-html:
    vx cargo llvm-cov --workspace --html

ci:
    vx just fmt-check
    vx just check
    vx just lint
    vx just test

build-release *args:
    vx cargo build --release {{args}}

build-release-server target:
    vx cargo build --release -p agentverse-server --target {{target}}

build-release-cli target:
    vx cargo build --release -p agentverse --target {{target}}

build-release-target target:
    vx just build-release-server {{target}}
    vx just build-release-cli {{target}}

run-server *args:
    vx cargo run -p agentverse-server -- {{args}}

run-cli *args:
    vx cargo run -p agentverse -- {{args}}

docker-build:
    docker build -t agentverse-server:dev .

docker-up:
    docker compose up -d

docker-down:
    docker compose down

package-skills:
    vx python scripts/package_openclaw_skill.py

