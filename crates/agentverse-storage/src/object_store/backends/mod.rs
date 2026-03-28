pub mod custom;
pub mod github_release;
pub mod local;
pub mod s3;

pub use custom::CustomBackend;
pub use github_release::GitHubReleaseBackend;
pub use local::LocalDiskBackend;
pub use s3::S3Backend;
