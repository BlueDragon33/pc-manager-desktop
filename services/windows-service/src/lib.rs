//! Versioned, read-only protocol foundation for a future local Windows Service.
//!
//! This crate does NOT install or start a Windows Service, open an IPC listener,
//! authorize elevated operations, or accept remotely supplied machine commands.
//! No privileged operation may be added without authenticated local-caller IPC,
//! service-side policy enforcement, audit and rollback tests.

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_REQUEST_BYTES: usize = 4_096;
pub const MAX_REQUEST_ID_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRequest {
    pub protocol_version: u16,
    pub request_id: String,
    pub command: ServiceCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum ServiceCommand {
    GetServiceVersion,
    GetCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceReply {
    pub protocol_version: u16,
    pub request_id: String,
    pub result: ServiceResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ServiceResult {
    ServiceVersion {
        version: u16,
    },
    Capabilities {
        commands: Vec<&'static str>,
        privileged_operations_enabled: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    EmptyFrame,
    FrameTooLarge,
    MalformedRequest,
    UnsupportedVersion,
    InvalidRequestId,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyFrame => write!(f, "empty service protocol frame"),
            Self::FrameTooLarge => write!(f, "service protocol frame exceeds size limit"),
            Self::MalformedRequest => write!(f, "malformed or unsupported service command"),
            Self::UnsupportedVersion => write!(f, "incompatible service protocol version"),
            Self::InvalidRequestId => write!(f, "invalid service request identifier"),
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Parse only a bounded, versioned, strongly typed read-only request.
/// Caller authentication belongs at the future IPC transport boundary.
pub fn parse_request(frame: &[u8]) -> Result<ServiceRequest, ProtocolError> {
    if frame.is_empty() {
        return Err(ProtocolError::EmptyFrame);
    }
    if frame.len() > MAX_REQUEST_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }

    let request: ServiceRequest =
        serde_json::from_slice(frame).map_err(|_| ProtocolError::MalformedRequest)?;
    if request.protocol_version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion);
    }
    if request.request_id.is_empty()
        || request.request_id.len() > MAX_REQUEST_ID_LEN
        || !request
            .request_id
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    {
        return Err(ProtocolError::InvalidRequestId);
    }
    Ok(request)
}

/// Pure, read-only reply construction. Deliberately no filesystem, registry,
/// process, service-control, elevation, or remote-authority side effects.
pub fn handle_read_only_request(frame: &[u8]) -> Result<ServiceReply, ProtocolError> {
    let request = parse_request(frame)?;
    let result = match request.command {
        ServiceCommand::GetServiceVersion => ServiceResult::ServiceVersion {
            version: PROTOCOL_VERSION,
        },
        ServiceCommand::GetCapabilities => ServiceResult::Capabilities {
            commands: vec!["GET_SERVICE_VERSION", "GET_CAPABILITIES"],
            privileged_operations_enabled: false,
        },
    };

    Ok(ServiceReply {
        protocol_version: PROTOCOL_VERSION,
        request_id: request.request_id,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSION: &[u8] = br#"{"protocol_version":1,"request_id":"req_1","command":{"type":"GET_SERVICE_VERSION"}}"#;
    const CAPABILITIES: &[u8] = br#"{"protocol_version":1,"request_id":"req-2","command":{"type":"GET_CAPABILITIES"}}"#;

    #[test]
    fn version_and_capabilities_are_read_only() {
        let version = handle_read_only_request(VERSION).expect("valid version command");
        assert_eq!(version.request_id, "req_1");
        assert_eq!(
            version.result,
            ServiceResult::ServiceVersion {
                version: PROTOCOL_VERSION
            }
        );
        let capabilities =
            handle_read_only_request(CAPABILITIES).expect("valid capabilities command");
        match capabilities.result {
            ServiceResult::Capabilities {
                commands,
                privileged_operations_enabled,
            } => {
                assert_eq!(commands, vec!["GET_SERVICE_VERSION", "GET_CAPABILITIES"]);
                assert!(!privileged_operations_enabled);
            }
            _ => panic!("unexpected capabilities result"),
        }
    }

    #[test]
    fn unsupported_mutating_and_remote_commands_fail_closed() {
        for name in ["EXECUTE_SHELL", "RUN_POWERSHELL", "EXECUTE_CLEANUP_PLAN", "SET_STARTUP_ENTRY"] {
            let frame = format!(
                r#"{{"protocol_version":1,"request_id":"req","command":{{"type":"{name}"}}}}"#
            );
            assert_eq!(
                parse_request(frame.as_bytes()),
                Err(ProtocolError::MalformedRequest)
            );
        }
    }

    #[test]
    fn mismatched_version_is_rejected() {
        let frame = br#"{"protocol_version":2,"request_id":"req","command":{"type":"GET_CAPABILITIES"}}"#;
        assert_eq!(parse_request(frame), Err(ProtocolError::UnsupportedVersion));
    }

    #[test]
    fn empty_oversize_and_malformed_frames_are_rejected() {
        assert_eq!(parse_request(b""), Err(ProtocolError::EmptyFrame));
        assert_eq!(
            parse_request(&vec![b'x'; MAX_REQUEST_BYTES + 1]),
            Err(ProtocolError::FrameTooLarge)
        );
        assert_eq!(
            parse_request(br#"{"protocol_version":1"#),
            Err(ProtocolError::MalformedRequest)
        );
    }

    #[test]
    fn unexpected_fields_and_argument_injection_are_rejected() {
        let unknown_root = br#"{"protocol_version":1,"request_id":"req","command":{"type":"GET_CAPABILITIES"},"elevate":true}"#;
        assert_eq!(
            parse_request(unknown_root),
            Err(ProtocolError::MalformedRequest)
        );
        let command_args = br#"{"protocol_version":1,"request_id":"req","command":{"type":"GET_CAPABILITIES","path":"C:/Windows"}}"#;
        assert_eq!(
            parse_request(command_args),
            Err(ProtocolError::MalformedRequest)
        );
    }

    #[test]
    fn request_identifiers_are_strictly_bounded() {
        let invalid = br#"{"protocol_version":1,"request_id":"../../token","command":{"type":"GET_CAPABILITIES"}}"#;
        assert_eq!(parse_request(invalid), Err(ProtocolError::InvalidRequestId));
        let excessive = format!(
            r#"{{"protocol_version":1,"request_id":"{}","command":{{"type":"GET_CAPABILITIES"}}}}"#,
            "a".repeat(MAX_REQUEST_ID_LEN + 1)
        );
        assert_eq!(
            parse_request(excessive.as_bytes()),
            Err(ProtocolError::InvalidRequestId)
        );
    }
}
