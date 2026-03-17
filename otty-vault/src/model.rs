use std::cmp::Ordering;

use secrecy::{ExposeSecret, SecretSlice};
use uuid::Uuid;

use crate::errors::{VaultError, VaultResult};

/// Runtime lock status of an opened vault handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultStatus {
    /// The handle is locked and cannot serve read/write operations.
    Locked,
    /// The handle is unlocked and can serve read/write operations.
    Unlocked,
}

/// Typed identifier for folders in the vault tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FolderId(Uuid);

impl FolderId {
    /// Create a new random folder identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a typed folder identifier from a raw UUID.
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

impl Default for FolderId {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed identifier for secrets in the vault tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SecretId(Uuid);

impl SecretId {
    /// Create a new random secret identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a typed secret identifier from a raw UUID.
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

/// Supported secret kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// Generic password-like secret.
    Password,
}

/// Tree node for a folder and all nested metadata nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderTreeNode {
    folder_id: FolderId,
    name: String,
    children: Vec<VaultMetadataNode>,
}

impl FolderTreeNode {
    /// Create a folder tree node from explicit fields.
    pub fn new(
        folder_id: FolderId,
        name: impl Into<String>,
        children: Vec<VaultMetadataNode>,
    ) -> VaultResult<Self> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self {
            folder_id,
            name,
            children,
        })
    }

    /// Typed identifier of this folder.
    pub fn folder_id(&self) -> FolderId {
        self.folder_id
    }

    /// Display name of this folder.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Full list of direct child nodes.
    pub fn children(&self) -> &[VaultMetadataNode] {
        &self.children
    }
}

/// Tree node containing secret metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretTreeNode {
    secret_id: SecretId,
    metadata: SecretMetadata,
}

impl SecretTreeNode {
    /// Create a secret tree node from explicit fields.
    pub fn new(secret_id: SecretId, metadata: SecretMetadata) -> Self {
        Self {
            secret_id,
            metadata,
        }
    }

    /// Typed identifier of this secret.
    pub fn secret_id(&self) -> SecretId {
        self.secret_id
    }

    /// Non-sensitive metadata for this secret.
    pub fn metadata(&self) -> &SecretMetadata {
        &self.metadata
    }
}

/// Unified node used by the vault sidebar tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VaultMetadataNode {
    /// Folder branch node.
    Folder(FolderTreeNode),
    /// Secret leaf node.
    Secret(SecretTreeNode),
}

impl VaultMetadataNode {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Folder(folder) => folder.name(),
            Self::Secret(secret) => secret.metadata().name(),
        }
    }
}

/// Full vault metadata snapshot, including the root folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultMetadataTree {
    root: FolderTreeNode,
}

impl VaultMetadataTree {
    /// Create a metadata tree with an explicit root folder.
    pub fn new(root: FolderTreeNode) -> Self {
        Self { root }
    }

    /// Root folder node for the current vault.
    pub fn root(&self) -> &FolderTreeNode {
        &self.root
    }
}

/// Sensitive bytes container used for secret payloads.
#[derive(Debug, Default)]
pub struct SecretBytes(SecretSlice<u8>);

impl SecretBytes {
    /// Construct secret bytes from an owned byte buffer.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(SecretSlice::from(bytes))
    }

    /// Borrow secret bytes without exposing ownership.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.expose_secret()
    }

    /// Whether the secret payload is empty.
    pub fn is_empty(&self) -> bool {
        self.as_bytes().is_empty()
    }

    /// Length in bytes of the payload.
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Copy the payload into an owned byte buffer.
    pub fn into_inner(self) -> Vec<u8> {
        self.0.expose_secret().to_vec()
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

/// Metadata that describes a secret without exposing secret value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMetadata {
    folder_id: FolderId,
    name: String,
    username: Option<String>,
    kind: SecretKind,
    tags: Vec<String>,
}

impl SecretMetadata {
    /// Create secret metadata.
    pub fn new(
        folder_id: FolderId,
        name: impl Into<String>,
        username: Option<String>,
        kind: SecretKind,
        tags: Vec<String>,
    ) -> VaultResult<Self> {
        let name = name.into();
        validate_name(&name)?;
        Ok(Self {
            folder_id,
            name,
            username,
            kind,
            tags,
        })
    }

    /// Folder identifier where this secret is located.
    pub fn folder_id(&self) -> FolderId {
        self.folder_id
    }

    /// Visible secret name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Optional username field.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Secret classification.
    pub fn kind(&self) -> SecretKind {
        self.kind
    }

    /// User-defined tags.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Update metadata folder target.
    pub fn set_folder_id(&mut self, folder_id: FolderId) {
        self.folder_id = folder_id;
    }

    /// Update secret name with validation.
    pub fn set_name(&mut self, name: impl Into<String>) -> VaultResult<()> {
        let next = name.into();
        validate_name(&next)?;
        self.name = next;
        Ok(())
    }

    /// Update the optional username field.
    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    /// Replace tags with a new list.
    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
    }

    pub(crate) fn validate(&self) -> VaultResult<()> {
        validate_name(&self.name)
    }
}

/// Distinguishes metadata-only updates from payload replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretValueUpdate {
    /// Keep existing encrypted payload unchanged.
    Keep,
    /// Replace payload with a new value.
    Set(SecretBytes),
}

/// Search result containing a secret id, metadata and computed path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSearchItem {
    secret_id: SecretId,
    metadata: SecretMetadata,
    path: String,
}

impl SecretSearchItem {
    /// Create a secret search item.
    pub fn new(
        secret_id: SecretId,
        metadata: SecretMetadata,
        path: impl Into<String>,
    ) -> Self {
        Self {
            secret_id,
            metadata,
            path: path.into(),
        }
    }

    /// Typed identifier for the found secret.
    pub fn secret_id(&self) -> SecretId {
        self.secret_id
    }

    /// Secret metadata payload.
    pub fn metadata(&self) -> &SecretMetadata {
        &self.metadata
    }

    /// Computed human-readable path in the folder tree.
    pub fn path(&self) -> &str {
        &self.path
    }
}

pub(crate) fn validate_name(name: &str) -> VaultResult<()> {
    if normalized_name(name).is_empty() {
        return Err(VaultError::InvalidSettings);
    }

    Ok(())
}

pub(crate) fn normalized_name(name: &str) -> &str {
    name.trim()
}

pub(crate) fn names_conflict(left: &str, right: &str) -> bool {
    normalized_name(left) == normalized_name(right)
}

pub(crate) fn sort_metadata_nodes(nodes: &mut [VaultMetadataNode]) {
    nodes.sort_by(compare_metadata_nodes);
}

fn compare_metadata_nodes(
    left: &VaultMetadataNode,
    right: &VaultMetadataNode,
) -> Ordering {
    match (left, right) {
        (VaultMetadataNode::Folder(_), VaultMetadataNode::Secret(_)) => {
            Ordering::Less
        },
        (VaultMetadataNode::Secret(_), VaultMetadataNode::Folder(_)) => {
            Ordering::Greater
        },
        _ => left.name().cmp(right.name()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FolderId, SecretBytes, SecretId, SecretKind, SecretMetadata,
        SecretSearchItem, SecretValueUpdate, VaultStatus, names_conflict,
        validate_name,
    };
    use crate::errors::VaultError;

    #[test]
    fn vault_status_has_locked_and_unlocked_states() {
        assert_ne!(VaultStatus::Locked, VaultStatus::Unlocked);
    }

    #[test]
    fn folder_name_validation_rejects_blank_and_whitespace() {
        assert_eq!(validate_name(""), Err(VaultError::InvalidSettings));
        assert_eq!(validate_name("   "), Err(VaultError::InvalidSettings));
    }

    #[test]
    fn secret_name_validation_rejects_blank_and_whitespace() {
        let folder_id = FolderId::new();
        let blank = SecretMetadata::new(
            folder_id,
            "",
            None,
            SecretKind::Password,
            Vec::new(),
        );
        assert_eq!(blank, Err(VaultError::InvalidSettings));

        let spaces = SecretMetadata::new(
            folder_id,
            "   ",
            None,
            SecretKind::Password,
            Vec::new(),
        );
        assert_eq!(spaces, Err(VaultError::InvalidSettings));
    }

    #[test]
    fn duplicate_check_uses_trimmed_names() {
        assert!(names_conflict("foo", " foo "));
        assert!(!names_conflict("foo", "bar"));
    }

    #[test]
    fn folder_and_secret_ids_are_distinct_types() {
        use std::any::TypeId;

        assert_ne!(TypeId::of::<FolderId>(), TypeId::of::<SecretId>(),);
    }

    #[test]
    fn secret_kind_first_version_contains_password() {
        let kinds = [SecretKind::Password];
        assert_eq!(kinds.len(), 1);
    }

    #[test]
    fn secret_bytes_debug_does_not_leak_payload() {
        let secret = SecretBytes::new(b"super-secret".to_vec());
        let debug = format!("{secret:?}");
        assert!(!debug.contains("super-secret"));
    }

    #[test]
    fn keep_and_set_empty_are_different_update_states() {
        assert_ne!(
            SecretValueUpdate::Keep,
            SecretValueUpdate::Set(SecretBytes::default()),
        );
    }

    #[test]
    fn secret_search_item_keeps_secret_id_outside_metadata() {
        let folder_id = FolderId::new();
        let metadata = SecretMetadata::new(
            folder_id,
            "api-token",
            Some("bot".to_string()),
            SecretKind::Password,
            vec!["ci".to_string()],
        )
        .expect("metadata must be valid");
        let secret_id = SecretId::new();
        let item = SecretSearchItem::new(
            secret_id,
            metadata.clone(),
            "Root/api-token",
        );

        assert_eq!(item.secret_id(), secret_id);
        assert_eq!(item.metadata(), &metadata);
        assert_eq!(item.path(), "Root/api-token");
    }
}
