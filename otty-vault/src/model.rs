use std::fmt;

use secrecy::{ExposeSecret, SecretSlice};
use uuid::Uuid;

use crate::errors::{VaultError, VaultResult};

/// Runtime state of an opened vault handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultState {
    /// The handle exists but is currently locked.
    Locked,
    /// The handle is unlocked and can serve read/write operations.
    Unlocked,
}

/// Typed identifier for a secret group.
///
/// # Examples
///
/// ```
/// use otty_vault::SecretGroupId;
///
/// let group_id = SecretGroupId::new();
/// assert_eq!(group_id, SecretGroupId::from_uuid(group_id.into_uuid()));
/// ```
///
/// ```compile_fail
/// use otty_vault::{SecretGroupId, SecretId};
///
/// fn takes_secret(_: SecretId) {}
///
/// let group_id = SecretGroupId::new();
/// takes_secret(group_id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SecretGroupId(Uuid);

impl SecretGroupId {
    /// Create a new random secret-group identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wrap a raw UUID in the secret-group newtype.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Borrow the raw UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Consume the wrapper and return the raw UUID.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for SecretGroupId {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed identifier for a secret.
///
/// # Examples
///
/// ```
/// use otty_vault::SecretId;
///
/// let secret_id = SecretId::new();
/// assert_eq!(secret_id, SecretId::from_uuid(secret_id.into_uuid()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SecretId(Uuid);

impl SecretId {
    /// Create a new random secret identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wrap a raw UUID in the secret newtype.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Borrow the raw UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Consume the wrapper and return the raw UUID.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for SecretId {
    fn default() -> Self {
        Self::new()
    }
}

/// Supported secret kinds for the first vault revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// Password-like credential material.
    Password,
}

/// Sensitive bytes container used for secret payloads.
///
/// # Examples
///
/// ```
/// use otty_vault::SecretBytes;
///
/// let secret = SecretBytes::from(vec![1, 2, 3]);
/// assert_eq!(secret.len(), 3);
/// assert_eq!(secret.as_bytes(), &[1, 2, 3]);
/// ```
#[derive(Default)]
pub struct SecretBytes(SecretSlice<u8>);

impl SecretBytes {
    /// Construct secret bytes from an owned byte buffer.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(SecretSlice::from(bytes))
    }

    /// Borrow the payload bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.expose_secret()
    }

    /// Return `true` when the payload is empty.
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    /// Return the payload length in bytes.
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Copy the payload into a fresh owned buffer.
    pub fn into_inner(self) -> Vec<u8> {
        self.0.expose_secret().to_vec()
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretBytes([REDACTED])")
    }
}

impl Clone for SecretBytes {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl PartialEq for SecretBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for SecretBytes {}

impl From<Vec<u8>> for SecretBytes {
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}

/// Metadata that describes a secret without exposing its payload or identity.
///
/// # Examples
///
/// ```
/// use otty_vault::{SecretGroupId, SecretKind, SecretMetadata};
///
/// let metadata = SecretMetadata::new(
///     SecretGroupId::new(),
///     "ssh-prod",
///     Some(String::from("root")),
///     SecretKind::Password,
///     vec![String::from("infra")],
/// )
/// .expect("metadata should be valid");
///
/// assert_eq!(metadata.name(), "ssh-prod");
/// assert_eq!(metadata.username(), Some("root"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMetadata {
    secret_group_id: SecretGroupId,
    name: String,
    username: Option<String>,
    kind: SecretKind,
    tags: Vec<String>,
}

impl SecretMetadata {
    /// Build secret metadata with validated non-empty name.
    pub fn new(
        secret_group_id: SecretGroupId,
        name: impl Into<String>,
        username: Option<String>,
        kind: SecretKind,
        tags: Vec<String>,
    ) -> VaultResult<Self> {
        let name = name.into();
        validate_name(&name)?;

        Ok(Self {
            secret_group_id,
            name,
            username,
            kind,
            tags,
        })
    }

    /// Return the owning secret-group identifier.
    pub fn secret_group_id(&self) -> SecretGroupId {
        self.secret_group_id
    }

    /// Return the visible secret name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the optional username field.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Return the secret classification.
    pub fn kind(&self) -> SecretKind {
        self.kind
    }

    /// Return the secret tags.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Move this metadata to a different secret group.
    pub fn move_to_secret_group(&mut self, secret_group_id: SecretGroupId) {
        self.secret_group_id = secret_group_id;
    }

    /// Rename the secret while preserving the other metadata fields.
    pub fn rename(&mut self, name: impl Into<String>) -> VaultResult<()> {
        let name = name.into();
        validate_name(&name)?;
        self.name = name;
        Ok(())
    }

    /// Replace the optional username field.
    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    /// Replace the current tags.
    pub fn replace_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
    }
}

/// Secret identity paired with non-sensitive metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    id: SecretId,
    metadata: SecretMetadata,
}

impl Secret {
    /// Build a secret from an identifier and metadata.
    pub fn new(id: SecretId, metadata: SecretMetadata) -> Self {
        Self { id, metadata }
    }

    /// Return the secret identifier.
    pub fn id(&self) -> SecretId {
        self.id
    }

    /// Return the non-sensitive secret metadata.
    pub fn metadata(&self) -> &SecretMetadata {
        &self.metadata
    }
}

/// Top-level secret group with all secrets that belong to it.
///
/// # Examples
///
/// ```
/// use otty_vault::{SecretGroup, SecretGroupId};
///
/// let group = SecretGroup::new(SecretGroupId::new(), "infra", Vec::new())
///     .expect("group should be valid");
///
/// assert_eq!(group.name(), "infra");
/// assert!(group.secrets().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretGroup {
    id: SecretGroupId,
    name: String,
    secrets: Vec<Secret>,
}

impl SecretGroup {
    /// Build a secret group and normalize the secret ordering by name.
    pub fn new(
        id: SecretGroupId,
        name: impl Into<String>,
        mut secrets: Vec<Secret>,
    ) -> VaultResult<Self> {
        let name = name.into();
        validate_name(&name)?;
        sort_secrets(&mut secrets);

        Ok(Self { id, name, secrets })
    }

    /// Return the secret-group identifier.
    pub fn id(&self) -> SecretGroupId {
        self.id
    }

    /// Return the visible secret-group name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the group secrets in deterministic order.
    pub fn secrets(&self) -> &[Secret] {
        &self.secrets
    }
}

/// Search result used by UI flows that need the secret group name separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSearchItem {
    secret: Secret,
    secret_group_name: String,
}

impl SecretSearchItem {
    /// Build a search result item.
    pub fn new(
        secret: Secret,
        secret_group_name: impl Into<String>,
    ) -> VaultResult<Self> {
        let secret_group_name = secret_group_name.into();
        validate_name(&secret_group_name)?;

        Ok(Self {
            secret,
            secret_group_name,
        })
    }

    /// Return the full secret.
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    /// Return the owning secret-group name.
    pub fn secret_group_name(&self) -> &str {
        &self.secret_group_name
    }
}

/// Explicitly distinguish between keeping and replacing a secret payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretValueUpdate {
    /// Preserve the existing payload bytes.
    Keep,
    /// Replace the payload bytes with a new value.
    Set(SecretBytes),
}

pub(crate) fn validate_name(name: &str) -> VaultResult<()> {
    if name.trim().is_empty() {
        return Err(VaultError::InvalidName);
    }

    Ok(())
}

pub(crate) fn names_conflict(existing: &str, candidate: &str) -> bool {
    normalize_name(existing) == normalize_name(candidate)
}

pub(crate) fn sort_secret_groups(secret_groups: &mut [SecretGroup]) {
    secret_groups.sort_by(|left, right| {
        left.name()
            .cmp(right.name())
            .then_with(|| left.id().cmp(&right.id()))
    });
}

fn sort_secrets(secrets: &mut [Secret]) {
    secrets.sort_by(|left, right| {
        left.metadata()
            .name()
            .cmp(right.metadata().name())
            .then_with(|| left.id().cmp(&right.id()))
    });
}

fn normalize_name(name: &str) -> &str {
    name.trim()
}

#[cfg(test)]
mod tests {
    use super::{
        Secret, SecretBytes, SecretGroup, SecretGroupId, SecretId, SecretKind,
        SecretMetadata, SecretSearchItem, SecretValueUpdate, VaultState,
        names_conflict,
    };
    use crate::VaultError;

    #[test]
    fn vault_state_covers_locked_and_unlocked() {
        assert!(matches!(VaultState::Locked, VaultState::Locked));
        assert!(matches!(VaultState::Unlocked, VaultState::Unlocked));
    }

    #[test]
    fn secret_group_name_validation_rejects_empty_or_whitespace_names() {
        let empty = SecretGroup::new(SecretGroupId::new(), "", Vec::new());
        let whitespace =
            SecretGroup::new(SecretGroupId::new(), "   ", Vec::new());

        assert_eq!(empty, Err(VaultError::InvalidName));
        assert_eq!(whitespace, Err(VaultError::InvalidName));
    }

    #[test]
    fn secret_name_validation_rejects_empty_or_whitespace_names() {
        let group_id = SecretGroupId::new();
        let empty = SecretMetadata::new(
            group_id,
            "",
            None,
            SecretKind::Password,
            Vec::new(),
        );
        let whitespace = SecretMetadata::new(
            group_id,
            "   ",
            None,
            SecretKind::Password,
            Vec::new(),
        );

        assert_eq!(empty, Err(VaultError::InvalidName));
        assert_eq!(whitespace, Err(VaultError::InvalidName));
    }

    #[test]
    fn secret_group_duplicate_check_uses_trimmed_names() {
        assert!(names_conflict("foo", " foo "));
    }

    #[test]
    fn secret_duplicate_check_uses_trimmed_names_within_group() {
        assert!(names_conflict("database", " database "));
    }

    #[test]
    fn secret_kind_is_password_only_in_v1() {
        assert!(matches!(SecretKind::Password, SecretKind::Password));
    }

    #[test]
    fn secret_bytes_debug_is_redacted() {
        let secret = SecretBytes::from(b"super-secret".to_vec());
        let debug = format!("{secret:?}");

        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("REDACTED"));
    }

    #[test]
    fn keep_and_set_empty_are_distinct_secret_value_update_states() {
        assert_ne!(
            SecretValueUpdate::Keep,
            SecretValueUpdate::Set(SecretBytes::default())
        );
    }

    #[test]
    fn secret_metadata_stays_separate_from_secret_identity_and_value() {
        let group_id = SecretGroupId::new();
        let metadata = SecretMetadata::new(
            group_id,
            "ssh-prod",
            Some(String::from("root")),
            SecretKind::Password,
            vec![String::from("ops")],
        )
        .expect("metadata should be valid");
        let secret = Secret::new(SecretId::new(), metadata.clone());

        assert_eq!(secret.metadata(), &metadata);
        assert_eq!(secret.metadata().secret_group_id(), group_id);
    }

    #[test]
    fn secret_search_item_keeps_secret_and_secret_group_name_separately() {
        let group_id = SecretGroupId::new();
        let metadata = SecretMetadata::new(
            group_id,
            "ssh-prod",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");
        let secret = Secret::new(SecretId::new(), metadata);
        let search_item = SecretSearchItem::new(secret.clone(), "infra")
            .expect("search item should be valid");

        assert_eq!(search_item.secret(), &secret);
        assert_eq!(search_item.secret_group_name(), "infra");
    }

    #[test]
    fn secret_metadata_mutators_update_fields() {
        let group_id = SecretGroupId::new();
        let next_group_id = SecretGroupId::new();
        let mut metadata = SecretMetadata::new(
            group_id,
            "ssh-prod",
            None,
            SecretKind::Password,
            Vec::new(),
        )
        .expect("metadata should be valid");

        metadata.move_to_secret_group(next_group_id);
        metadata
            .rename("ssh-stage")
            .expect("renaming should be valid");
        metadata.set_username(Some(String::from("deployer")));
        metadata.replace_tags(vec![String::from("stage")]);

        assert_eq!(metadata.secret_group_id(), next_group_id);
        assert_eq!(metadata.name(), "ssh-stage");
        assert_eq!(metadata.username(), Some("deployer"));
        assert_eq!(metadata.tags(), &[String::from("stage")]);
    }

    #[test]
    fn secret_group_constructor_sorts_secrets_by_name() {
        let group_id = SecretGroupId::new();
        let zeta = Secret::new(
            SecretId::new(),
            SecretMetadata::new(
                group_id,
                "zeta",
                None,
                SecretKind::Password,
                Vec::new(),
            )
            .expect("metadata should be valid"),
        );
        let alpha = Secret::new(
            SecretId::new(),
            SecretMetadata::new(
                group_id,
                "alpha",
                None,
                SecretKind::Password,
                Vec::new(),
            )
            .expect("metadata should be valid"),
        );

        let group = SecretGroup::new(group_id, "infra", vec![zeta, alpha])
            .expect("group should be valid");

        assert_eq!(group.secrets()[0].metadata().name(), "alpha");
        assert_eq!(group.secrets()[1].metadata().name(), "zeta");
    }

    #[test]
    fn secret_bytes_helpers_report_length_and_round_trip_contents() {
        let empty = SecretBytes::default();
        let payload = SecretBytes::from(b"payload".to_vec());

        assert!(empty.is_empty());
        assert_eq!(payload.len(), 7);
        assert_eq!(payload.clone().into_inner(), b"payload");
    }

    #[test]
    fn typed_ids_round_trip_raw_uuid_access() {
        let group_id = SecretGroupId::new();
        let secret_id = SecretId::new();

        assert_eq!(SecretGroupId::from_uuid(*group_id.as_uuid()), group_id);
        assert_eq!(SecretId::from_uuid(*secret_id.as_uuid()), secret_id);
    }
}
