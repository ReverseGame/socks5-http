use std::sync::Arc;

use acl::AclRule;
use auth::Authenticator;
use tokio::sync::RwLock;

pub mod acl;
pub mod auth;

pub type AuthCenter = Arc<RwLock<dyn Authenticator + Send + Sync>>;
pub type AclCenter = Arc<RwLock<dyn AclRule + Send + Sync>>;
