use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("node not found: {0}")]
    NodeNotFound(Uuid),
    #[error("node already exists: {0}")]
    DuplicateNode(Uuid),
    #[error("edge already exists between {from_id} and {to_id}")]
    DuplicateEdge { from_id: Uuid, to_id: Uuid },
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error(transparent)]
    Graph(#[from] GraphError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error("invalid node type: {0}")]
    InvalidNodeType(String),
    #[error("invalid edge type: {0}")]
    InvalidEdgeType(String),
    #[error("invalid focus depth: {0}")]
    InvalidFocusDepth(String),
    #[error("invalid change type: {0}")]
    InvalidChangeType(String),
    #[error("invalid arc type: {0}")]
    InvalidArcType(String),
    #[error("invalid contract state: {0}")]
    InvalidContractState(String),
    #[error("invalid datetime: {0}")]
    InvalidDateTime(String),
}

#[derive(Debug, Error)]
pub enum VaultError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error("argon2 failure: {0}")]
    Argon2(String),
    #[error(transparent)]
    PasswordHash(#[from] argon2::password_hash::Error),
    #[error("vault is locked")]
    Locked,
    #[error("invalid vault password")]
    InvalidPassword,
    #[error("ciphertext is malformed")]
    MalformedCiphertext,
    #[error("encryption failure")]
    EncryptionFailure,
    #[error("decryption failure")]
    DecryptionFailure,
}

#[derive(Debug, Error)]
pub enum SurrealStoreError {
    #[error(transparent)]
    Surreal(#[from] surrealdb::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
