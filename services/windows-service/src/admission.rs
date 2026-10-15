//! P15B1: fail-closed read-only admission and replay policy.
//!
//! This is deliberately NOT an IPC transport or a token authenticator. Only a
//! future server-side Windows token verifier may create a VerifiedCaller.
//! Until that verifier and a local-only named pipe exist, the service binary
//! remains inactive and no untrusted client can reach this admission gate.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::{handle_read_only_request, parse_request, ProtocolError, ServiceReply};

pub const MAX_IN_FLIGHT_IDENTIFIERS: usize = 256;
pub const REPLAY_WINDOW: Duration = Duration::from_secs(60);
pub const MAX_REPLY_BYTES: usize = 4_096;

/// Authenticated evidence MUST originate from a Windows server-side caller
/// token, never from a request field, PID, claimed SID or frontend parameter.
/// Private fields and the absent production constructor are intentional.
#[derive(Debug)]
pub struct VerifiedCaller {
    sid: String,
    is_local: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionError {
    InvalidOwner,
    InvalidCaller,
    CrossUser,
    RemoteSession,
    Replay,
    CapacityExceeded,
    Protocol(ProtocolError),
    ReplyTooLarge,
}

impl From<ProtocolError> for AdmissionError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

/// One per-service-instance admission budget; never trust request-id claims
/// as proof of Windows identity. The transport must also bound connections,
/// client read timeouts and concurrent requests independently.
#[derive(Debug)]
pub struct ReadOnlyAdmissionGate {
    owner_sid: String,
    recently_seen: HashMap<String, Instant>,
}

fn valid_canonical_sid(sid: &str) -> bool {
    if sid.len() > 184 || !sid.starts_with("S-1-") {
        return false;
    }
    let components: Vec<_> = sid.split('-').collect();
    components.len() >= 4
        && components[2..].iter().all(|part| {
            !part.is_empty()
                && part.len() <= 20
                && part.bytes().all(|ch| ch.is_ascii_digit())
        })
}

impl ReadOnlyAdmissionGate {
    /// `owner_sid` must be read from trusted local service configuration,
    /// not from a request frame or App Manager control plane.
    pub fn new(owner_sid: String) -> Result<Self, AdmissionError> {
        if !valid_canonical_sid(&owner_sid) {
            return Err(AdmissionError::InvalidOwner);
        }
        Ok(Self {
            owner_sid,
            recently_seen: HashMap::new(),
        })
    }

    /// Admission is intentionally not exposed through a public constructor
    /// for VerifiedCaller. The transport is not active in P15B1.
    pub fn handle_read_only(
        &mut self,
        caller: &VerifiedCaller,
        frame: &[u8],
    ) -> Result<ServiceReply, AdmissionError> {
        self.handle_at(caller, frame, Instant::now())
    }

    fn handle_at(
        &mut self,
        caller: &VerifiedCaller,
        frame: &[u8],
        now: Instant,
    ) -> Result<ServiceReply, AdmissionError> {
        if !valid_canonical_sid(&caller.sid) {
            return Err(AdmissionError::InvalidCaller);
        }
        if !caller.is_local {
            return Err(AdmissionError::RemoteSession);
        }
        if caller.sid != self.owner_sid {
            return Err(AdmissionError::CrossUser);
        }

        // Reject malformed frames before consuming replay budget.
        let request = parse_request(frame)?;
        self.recently_seen
            .retain(|_, accepted_at| now.saturating_duration_since(*accepted_at) < REPLAY_WINDOW);
        if self.recently_seen.contains_key(&request.request_id) {
            return Err(AdmissionError::Replay);
        }
        if self.recently_seen.len() >= MAX_IN_FLIGHT_IDENTIFIERS {
            return Err(AdmissionError::CapacityExceeded);
        }

        let reply = handle_read_only_request(frame)?;
        let encoded =
            serde_json::to_vec(&reply).map_err(|_| AdmissionError::ReplyTooLarge)?;
        if encoded.len() > MAX_REPLY_BYTES {
            return Err(AdmissionError::ReplyTooLarge);
        }
        // Record only successfully handled requests; fail closed on replay.
        self.recently_seen.insert(request.request_id, now);
        Ok(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceResult;

    const OWNER: &str = "S-1-5-21-100-200-300-1001";
    const OTHER: &str = "S-1-5-21-100-200-300-1002";

    fn trusted_test_caller(sid: &str, is_local: bool) -> VerifiedCaller {
        VerifiedCaller {
            sid: sid.to_owned(),
            is_local,
        }
    }

    fn frame(id: &str) -> Vec<u8> {
        format!(
            r#"{{"protocol_version":1,"request_id":"{id}","command":{{"type":"GET_CAPABILITIES"}}}}"#
        )
        .into_bytes()
    }

    #[test]
    fn accepted_owner_receives_only_read_only_capabilities() {
        let mut gate = ReadOnlyAdmissionGate::new(OWNER.into()).unwrap();
        let reply = gate
            .handle_read_only(&trusted_test_caller(OWNER, true), &frame("one"))
            .unwrap();
        assert!(matches!(
            reply.result,
            ServiceResult::Capabilities {
                privileged_operations_enabled: false,
                ..
            }
        ));
    }

    #[test]
    fn rejected_cross_user_and_remote_sessions_do_not_reserve_id() {
        let mut gate = ReadOnlyAdmissionGate::new(OWNER.into()).unwrap();
        assert_eq!(
            gate.handle_read_only(&trusted_test_caller(OTHER, true), &frame("same")),
            Err(AdmissionError::CrossUser)
        );
        assert_eq!(
            gate.handle_read_only(&trusted_test_caller(OWNER, false), &frame("same")),
            Err(AdmissionError::RemoteSession)
        );
        assert!(gate
            .handle_read_only(&trusted_test_caller(OWNER, true), &frame("same"))
            .is_ok());
    }

    #[test]
    fn replay_is_rejected_until_window_expires() {
        let start = Instant::now();
        let mut gate = ReadOnlyAdmissionGate::new(OWNER.into()).unwrap();
        let caller = trusted_test_caller(OWNER, true);
        assert!(gate.handle_at(&caller, &frame("replay"), start).is_ok());
        assert_eq!(
            gate.handle_at(&caller, &frame("replay"), start + Duration::from_secs(59)),
            Err(AdmissionError::Replay)
        );
        assert!(gate
            .handle_at(&caller, &frame("replay"), start + REPLAY_WINDOW)
            .is_ok());
    }

    #[test]
    fn budget_is_bounded_and_recovers_after_expiry() {
        let start = Instant::now();
        let mut gate = ReadOnlyAdmissionGate::new(OWNER.into()).unwrap();
        let caller = trusted_test_caller(OWNER, true);
        for id in 0..MAX_IN_FLIGHT_IDENTIFIERS {
            gate.handle_at(&caller, &frame(&format!("req{id}")), start)
                .unwrap();
        }
        assert_eq!(
            gate.handle_at(&caller, &frame("overflow"), start),
            Err(AdmissionError::CapacityExceeded)
        );
        assert!(gate
            .handle_at(&caller, &frame("overflow"), start + REPLAY_WINDOW)
            .is_ok());
    }

    #[test]
    fn invalid_versions_commands_and_frames_fail_without_consuming_budget() {
        let mut gate = ReadOnlyAdmissionGate::new(OWNER.into()).unwrap();
        let caller = trusted_test_caller(OWNER, true);
        let unsupported = br#"{"protocol_version":1,"request_id":"ok","command":{"type":"RUN_POWERSHELL"}}"#;
        assert_eq!(
            gate.handle_read_only(&caller, unsupported),
            Err(AdmissionError::Protocol(ProtocolError::MalformedRequest))
        );
        assert_eq!(
            gate.handle_read_only(&caller, &vec![0; 4097]),
            Err(AdmissionError::Protocol(ProtocolError::FrameTooLarge))
        );
        assert!(gate.handle_read_only(&caller, &frame("ok")).is_ok());
    }

    #[test]
    fn owner_and_peer_sids_are_strictly_canonical() {
        for bad in ["", "SYSTEM", "s-1-5-18", "S-1-5-evil", "S-1-", "S-1-5-18 extra"] {
            assert!(matches!(
                ReadOnlyAdmissionGate::new(bad.into()),
                Err(AdmissionError::InvalidOwner)
            ));
        }
        let mut gate = ReadOnlyAdmissionGate::new(OWNER.into()).unwrap();
        assert_eq!(
            gate.handle_read_only(&trusted_test_caller("administrator", true), &frame("id")),
            Err(AdmissionError::InvalidCaller)
        );
    }
}
