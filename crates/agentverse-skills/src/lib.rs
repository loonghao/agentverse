//! `agentverse-skills` — skill package backends, deploy logic, and publish hooks.
//!
//! ## Architecture
//!
//! ```text
//!   Publisher ──► API layer ──► HookRegistry ──► MetadataHook ──► DB
//!                                                └─ LoggingHook
//!
//!   Consumer ──► API layer ──► deploy::deploy_skill()
//!                              ├─ backend.download(url, tmp)
//!                              ├─ checksum verify
//!                              └─ extract_zip / extract_zip_subpath → agent paths
//! ```
//!
//! ## Supported backends
//!
//! | Backend            | SourceType    | Build URL?              | Pattern                    |
//! |--------------------|---------------|-------------------------|----------------------------|
//! | `ClawhubBackend`   | `clawhub`     | ✓ (namespace/name/ver)  | hub.openclaw.io release zip |
//! | `GitHubBackend`    | `github`      | ✓ (conventional)        | GitHub release asset        |
//! | `GitHubRepoBackend`| `github_repo` | ✗ (tree URL required)   | anthropics/skills pattern   |
//! | `UrlBackend`       | `url`         | ✗ (explicit URL only)   | any HTTP/HTTPS endpoint     |

pub mod backends;
pub mod deploy;
pub mod error;
pub mod hooks;
pub mod skill_md;

pub use backends::{
    parse_github_tree_url, ClawhubBackend, GitHubBackend, GitHubRepoBackend, GitHubRepoInfo,
    PackageBackend, UrlBackend,
};
pub use deploy::{
    agent_skills_root, all_known_agents, deploy_skill, extract_zip_subpath, skill_install_path,
};
pub use error::SkillError;
pub use hooks::{HookRegistry, LoggingHook, MetadataHook, PublishHook};
pub use skill_md::{parse_skill_md, ParsedSkillMd};
