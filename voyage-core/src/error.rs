//! Error types for Voyage Core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VoyageError {
    #[error("Core not initialized")]
    NotInitialized,

    #[error("Core already initialized")]
    AlreadyInitialized,

    #[error("Lock error")]
    LockError,

    #[error("Invalid packet: {0}")]
    InvalidPacket(String),

    #[error("Socket error: {0}")]
    SocketError(String),

    #[error("NAT table full")]
    NatTableFull,

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("NAT error: {0}")]
    Nat(String),

    #[error("Rule error: {0}")]
    Rule(String),

    #[error("SOCKS5 error: {0}")]
    Socks5Error(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, VoyageError>;

