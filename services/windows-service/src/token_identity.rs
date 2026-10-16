//! P15B3: Windows thread-token SID, impersonation-level and session evidence.
//!
//! Reads ONLY a genuine thread impersonation token. There is no process-token
//! fallback and no API accepting a caller-supplied SID/PID. A later named-pipe
//! listener must call ImpersonateNamedPipeClient and verify its pipe security
//! before obtaining a VerifiedCaller from this evidence. No pipe runs today.

use std::mem::size_of;
use std::ptr;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_NO_TOKEN, HANDLE,
};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{
    GetTokenInformation, SecurityImpersonation, TokenImpersonationLevel, TokenSessionId, TokenUser,
    TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};

use crate::admission::valid_canonical_sid;

const MAX_TOKEN_INFORMATION_BYTES: usize = 8_192;
const MAX_SID_UTF16_UNITS: usize = 184;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadTokenIdentity {
    sid: String,
    session_id: u32,
}

impl ThreadTokenIdentity {
    pub fn sid(&self) -> &str {
        &self.sid
    }

    pub fn session_id(&self) -> u32 {
        self.session_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenIdentityError {
    NoImpersonation,
    TokenUnavailable,
    InvalidTokenInformation,
    InsufficientImpersonationLevel,
    InvalidSession,
    InvalidSid,
}

struct TokenHandle(HANDLE);

impl Drop for TokenHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Reads a 32-bit token property from the current Windows token, not from
/// caller-provided JSON fields. Reject missing and unexpectedly sized values.
fn read_token_dword(token: HANDLE, information_class: i32) -> Result<u32, TokenIdentityError> {
    let mut value: u32 = 0;
    let mut returned: u32 = 0;
    let ok = unsafe {
        GetTokenInformation(
            token,
            information_class,
            (&mut value as *mut u32).cast(),
            size_of::<u32>() as u32,
            &mut returned,
        )
    };
    if ok == 0 || returned != size_of::<u32>() as u32 {
        return Err(TokenIdentityError::InvalidTokenInformation);
    }
    Ok(value)
}

/// The SID, token session ID and impersonation level all come from the SAME
/// current thread impersonation token. SecurityIdentification and delegation
/// tokens cannot be promoted into local service authorization.
pub fn current_thread_impersonation_identity() -> Result<ThreadTokenIdentity, TokenIdentityError> {
    let mut raw_token: HANDLE = ptr::null_mut();
    // SAFETY: GetCurrentThread returns a valid pseudohandle; token access is
    // read-only and the resulting real handle is closed by TokenHandle.
    let opened = unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut raw_token) };
    if opened == 0 {
        return if unsafe { GetLastError() } == ERROR_NO_TOKEN {
            Err(TokenIdentityError::NoImpersonation)
        } else {
            Err(TokenIdentityError::TokenUnavailable)
        };
    }
    if raw_token.is_null() {
        return Err(TokenIdentityError::TokenUnavailable);
    }
    let token = TokenHandle(raw_token);

    // SecurityIdentification tokens can report a SID but are not sufficient
    // for a future local IPC impersonation authorization decision.
    // SecurityDelegation is unnecessary for a machine-local service.
    let level = read_token_dword(token.0, TokenImpersonationLevel)?;
    if level != SecurityImpersonation as u32 {
        return Err(TokenIdentityError::InsufficientImpersonationLevel);
    }
    let session_id = read_token_dword(token.0, TokenSessionId)?;
    if session_id == u32::MAX {
        return Err(TokenIdentityError::InvalidSession);
    }

    let mut required: u32 = 0;
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if (required as usize) < size_of::<TOKEN_USER>()
        || (required as usize) > MAX_TOKEN_INFORMATION_BYTES
    {
        return Err(TokenIdentityError::InvalidTokenInformation);
    }

    // Vec<usize> provides at least pointer alignment for TOKEN_USER, unlike
    // a Vec<u8>. Keep the allocation alive while reading the embedded SID.
    let word_count = (required as usize).div_ceil(size_of::<usize>());
    let mut words = vec![0usize; word_count];
    let success = unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            words.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    };
    if success == 0 || (required as usize) > words.len() * size_of::<usize>() {
        return Err(TokenIdentityError::InvalidTokenInformation);
    }

    // SAFETY: TOKEN_USER was written by GetTokenInformation into aligned,
    // adequately sized storage and its Sid pointer remains valid until return.
    let user = unsafe { &*words.as_ptr().cast::<TOKEN_USER>() };
    if user.User.Sid.is_null() {
        return Err(TokenIdentityError::InvalidSid);
    }

    let mut allocated_sid: *mut u16 = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut allocated_sid) } == 0 {
        return Err(TokenIdentityError::InvalidSid);
    }
    if allocated_sid.is_null() {
        return Err(TokenIdentityError::InvalidSid);
    }

    // This string is OS-allocated by ConvertSidToStringSidW, never provided
    // in JSON by the frontend or a remote control plane.
    let result = unsafe {
        let mut length = 0usize;
        while length <= MAX_SID_UTF16_UNITS && *allocated_sid.add(length) != 0 {
            length += 1;
        }
        let value = if length == 0 || length > MAX_SID_UTF16_UNITS {
            Err(TokenIdentityError::InvalidSid)
        } else {
            String::from_utf16(std::slice::from_raw_parts(allocated_sid, length))
                .map_err(|_| TokenIdentityError::InvalidSid)
        };
        LocalFree(allocated_sid.cast());
        value
    }?;

    if !valid_canonical_sid(&result) {
        return Err(TokenIdentityError::InvalidSid);
    }
    Ok(ThreadTokenIdentity {
        sid: result,
        session_id,
    })
}

/// Compatibility helper that retains the strict level and session validation.
/// The returned SID alone still cannot authorize a named-pipe client.
pub fn current_thread_impersonation_sid() -> Result<String, TokenIdentityError> {
    current_thread_impersonation_identity().map(|identity| identity.sid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Security::{
        ImpersonateSelf, RevertToSelf, SecurityIdentification, SecurityImpersonation,
    };

    struct RevertGuard;
    impl Drop for RevertGuard {
        fn drop(&mut self) {
            unsafe {
                RevertToSelf();
            }
        }
    }

    #[test]
    fn refuses_to_fall_back_to_process_identity() {
        std::thread::spawn(|| {
            unsafe {
                RevertToSelf();
            }
            assert_eq!(
                current_thread_impersonation_sid(),
                Err(TokenIdentityError::NoImpersonation)
            );
        })
        .join()
        .expect("Windows token negative test thread");
    }

    #[test]
    fn identification_level_cannot_be_used_for_local_service_identity() {
        std::thread::spawn(|| {
            let ok = unsafe { ImpersonateSelf(SecurityIdentification) };
            assert_ne!(ok, 0, "fixture requires identification-only token");
            let _guard = RevertGuard;
            assert_eq!(
                current_thread_impersonation_identity(),
                Err(TokenIdentityError::InsufficientImpersonationLevel)
            );
        })
        .join()
        .expect("Windows identification-level token negative test");
    }

    #[test]
    fn obtains_canonical_sid_from_real_windows_impersonation_token() {
        let impersonated = unsafe { ImpersonateSelf(SecurityImpersonation) };
        assert_ne!(
            impersonated, 0,
            "Windows test fixture must impersonate self"
        );
        let _guard = RevertGuard;
        let identity =
            current_thread_impersonation_identity().expect("read genuine impersonation token");
        assert!(valid_canonical_sid(identity.sid()));
        // Both reads must come from the genuine Windows thread token.
        // There is no caller-provided session field and no process fallback.
        let second = current_thread_impersonation_identity().expect("read token again");
        assert_eq!(identity.session_id(), second.session_id());
        assert_eq!(identity.sid(), second.sid());
        assert_ne!(identity.session_id(), u32::MAX);
        assert_eq!(current_thread_impersonation_sid().unwrap(), identity.sid());
    }
}
