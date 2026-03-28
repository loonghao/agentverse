# Changelog

## [0.1.5](https://github.com/loonghao/agentverse/compare/v0.1.4...v0.1.5) (2026-03-28)


### Features

* add install scripts and CLI self-update command ([3fe7681](https://github.com/loonghao/agentverse/commit/3fe7681c64dc909b2caf75e8dc49e808866e024a))
* add shared updater crate, server self-update, and fix skills CI ([5b993f4](https://github.com/loonghao/agentverse/commit/5b993f49e1b493fac920e94440df1e7e21efff1e))
* add SKILL.md for ClawHub publish ([e806e69](https://github.com/loonghao/agentverse/commit/e806e693e70ddfa636be04775c6d1edbd6b06e9f))
* add skills management system with multi-backend support ([a38333f](https://github.com/loonghao/agentverse/commit/a38333fde95fe2722ede377896c6e96a3eac5902))
* full frontmatter parsing + new skills + ClawHub single-skill publish ([20d7f13](https://github.com/loonghao/agentverse/commit/20d7f1322c0bf3bc03c639e8b8b363a17e532ffa))


### Bug Fixes

* apply cargo fmt and clippy fixes; add pre-commit config ([e74d763](https://github.com/loonghao/agentverse/commit/e74d76341f6128beb89357b00abc17363d461652))
* **ci:** correct create-skill payload and make content optional ([8af86c6](https://github.com/loonghao/agentverse/commit/8af86c6c3ce96fc16230be4743d511496e9d8b57))
* **ci:** extract skill id from .artifact.id not .id ([813e7fc](https://github.com/loonghao/agentverse/commit/813e7fc6edf0874068fec6dd73ca8536c3f0cf58))
* **ci:** robust E2E server startup with log capture and crash detection ([8ee8a8d](https://github.com/loonghao/agentverse/commit/8ee8a8dfd5a067eaccdaf5ae569c6a0f47317902))
* **ci:** use pgvector/pgvector:pg17 image for E2E postgres service ([3e2d657](https://github.com/loonghao/agentverse/commit/3e2d65709c5e49b3fdf5f94dedc328f0669d2ff8))
* resolve CI failures in quality and validate-skills jobs ([1fcb637](https://github.com/loonghao/agentverse/commit/1fcb63706e00f951720c120438229905ad5588c7))
* resolve CLAWHUB_TOKEN check and rename skill to agentverse-cli ([55c70ba](https://github.com/loonghao/agentverse/commit/55c70ba8bb6871dde76de783a596ebc2d136d6e4))
* **test:** use concat! for OPENCLAW_SKILL to preserve YAML indentation ([b1cc393](https://github.com/loonghao/agentverse/commit/b1cc393404f9ef266a4baeb89138f8aa826ff6cd))

## [0.1.4](https://github.com/loonghao/agentverse/compare/v0.1.3...v0.1.4) (2026-03-28)


### Bug Fixes

* replace manual stub build with cargo-chef to fix Docker stage 2 failure ([b2679eb](https://github.com/loonghao/agentverse/commit/b2679eb25bdbb42a4c235d159ae7a8fdf94e2811))

## [0.1.3](https://github.com/loonghao/agentverse/compare/v0.1.2...v0.1.3) (2026-03-27)


### Bug Fixes

* add curl to builder stage and enable reqwest feature for utoipa-swagger-ui ([0b0614f](https://github.com/loonghao/agentverse/commit/0b0614f42a58b96cdeec1f506a88809be1e326b0))

## [0.1.2](https://github.com/loonghao/agentverse/compare/v0.1.1...v0.1.2) (2026-03-27)


### Features

* add Helm chart and fix release pipeline ([3cfb466](https://github.com/loonghao/agentverse/commit/3cfb46697cf0c965144dd243b641841441527083))


### Bug Fixes

* resolve all clippy warnings and update vx to 0.8.8 ([9c4e720](https://github.com/loonghao/agentverse/commit/9c4e72062db538237812a98bbf4c08de6fcc9cb7))

## [0.1.1](https://github.com/loonghao/agentverse/compare/v0.1.0...v0.1.1) (2026-03-27)


### Features

* initial release of AgentVerse — universal AI agent marketplace ([64d3e5e](https://github.com/loonghao/agentverse/commit/64d3e5e76493984a87830d84aa403f772c7f4c8b))
