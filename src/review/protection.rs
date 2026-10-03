//! The production trust origin's protection check (WRA-001).
//!
//! An enrollment is trusted only when the builder's effective token cannot
//! rewrite it: the file and every path component is owned by the host's
//! administrative principal and grants the builder's own token none of
//! write, append, delete, ownership, or permission change - checked by
//! `AccessCheck` against each path's own descriptor, so a writable parent
//! directory that would let the builder replace the file also refuses - no
//! component carries a reparse point, and the process token is not
//! elevated. On Unix the same rule reads as
//! root ownership with no group or other write bits. A fact that cannot be
//! verified is reported as exactly that - protection is never assumed, so
//! nothing a builder can write ever passes.

use std::path::Path;

/// Why a protection check refused. `NotProtected` is the caller's to
/// name - the enrollment manifest and the allowed-signers file each render
/// their own noun - while `Unverifiable` carries its fully rendered fact
/// line unchanged.
pub(crate) enum ProtectionRefusal {
    /// The file or an ancestor directory fails the protection rule.
    NotProtected,
    /// A fact could not be verified; carries the rendered detail line.
    Unverifiable(String),
}

/// Refuse unless `path` is protected against the builder actor `actor`.
pub(crate) fn require_protected(path: &Path, actor: &str) -> Result<(), ProtectionRefusal> {
    #[cfg(windows)]
    return windows::require_protected(path, actor);
    #[cfg(not(windows))]
    return unix::require_protected(path, actor);
}

#[cfg(windows)]
mod windows {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use crate::root::Displayed;

    use windows_sys::Win32::Foundation::{CloseHandle, LocalFree};
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        AccessCheck, DuplicateTokenEx, GetTokenInformation, MapGenericMask, SecurityImpersonation,
        TokenElevation, TokenImpersonation, DACL_SECURITY_INFORMATION, GENERIC_MAPPING,
        GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PRIVILEGE_SET,
        PSECURITY_DESCRIPTOR, PSID, TOKEN_DUPLICATE, TOKEN_ELEVATION, TOKEN_IMPERSONATE,
        TOKEN_QUERY,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ALL_ACCESS, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    };
    use windows_sys::Win32::System::SystemServices::MAXIMUM_ALLOWED;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// The Well-Known SIDs that may own a protected path: NT
    /// AUTHORITY\SYSTEM and BUILTIN\Administrators.
    const ADMINISTRATIVE_SIDS: [&str; 2] = ["S-1-5-18", "S-1-5-32-544"];
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    const FILE_WRITE_DATA: u32 = 0x0002;
    const FILE_APPEND_DATA: u32 = 0x0004;
    const FILE_ADD_FILE: u32 = 0x0002;
    const FILE_ADD_SUBDIRECTORY: u32 = 0x0004;
    const FILE_DELETE_CHILD: u32 = 0x0040;
    const FILE_WRITE_DAC: u32 = 0x40000;
    const FILE_WRITE_OWNER: u32 = 0x80000;
    const DELETE: u32 = 0x0001_0000;

    struct Descriptor {
        handle: *mut core::ffi::c_void,
        owner: PSID,
    }

    impl Drop for Descriptor {
        fn drop(&mut self) {
            // SAFETY: the descriptor was allocated by GetNamedSecurityInfoW
            // and is freed exactly once, here.
            unsafe { LocalFree(self.handle) };
        }
    }

    pub(crate) fn require_protected(
        path: &Path,
        _actor: &str,
    ) -> Result<(), super::ProtectionRefusal> {
        // The builder's identity here is its effective token, not its name:
        // AccessCheck below evaluates the token itself, so the passed name
        // is unused on this platform.
        // Every helper's error string is already a complete
        // "cannot be verified as protected" line; it passes through
        // unchanged, while every rule failure below is the caller's
        // `NotProtected` noun to render.
        let unverifiable = |line: String| super::ProtectionRefusal::Unverifiable(line);
        if elevated().map_err(unverifiable)? {
            return Err(unverifiable(format!(
                "the enrollment path {} cannot be verified as protected: the builder token \
                 is elevated",
                path.displayed()
            )));
        }
        // The reparse preflight walks the whole chain first: a path that
        // cannot be named through is unverifiable before any ownership
        // fact is judged, so a junction beats a builder-owned target.
        for component in chain(path).map_err(unverifiable)? {
            deny_reparse(path, &component).map_err(unverifiable)?;
            let descriptor = query(&component).map_err(unverifiable)?;
            let owner = sid_to_string(descriptor.owner).ok_or_else(|| {
                unverifiable(format!(
                    "the enrollment path {} cannot be verified as protected: the owner of \
                     {} cannot be named",
                    path.displayed(),
                    component.displayed()
                ))
            })?;
            if !ADMINISTRATIVE_SIDS.contains(&owner.as_str()) {
                return Err(super::ProtectionRefusal::NotProtected);
            }
            if !builder_cannot_write(&component, &descriptor).map_err(unverifiable)? {
                return Err(super::ProtectionRefusal::NotProtected);
            }
        }
        Ok(())
    }

    /// Every ancestor of `path` up to the root, `path` itself first. A
    /// reparse point anywhere refuses, named against the enrolled path
    /// itself so the refusal names the fact the runbook declared.
    fn chain(path: &Path) -> Result<Vec<std::path::PathBuf>, String> {
        let mut chain = Vec::new();
        let mut current = Some(path.to_path_buf());
        while let Some(step) = current {
            deny_reparse(path, &step)?;
            current = step.parent().map(std::path::PathBuf::from);
            chain.push(step);
        }
        Ok(chain)
    }

    /// Refuse when `step` - one component of the enrolled `path` - carries
    /// a reparse point or cannot be inspected at all.
    fn deny_reparse(path: &Path, step: &Path) -> Result<(), String> {
        use windows_sys::Win32::Storage::FileSystem::GetFileAttributesW;
        let wide: Vec<u16> = step
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 path buffer.
        let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
        if attributes == u32::MAX {
            return Err(format!(
                "the enrollment path {} cannot be verified as protected: {} cannot be \
                 inspected: {}",
                path.displayed(),
                step.displayed(),
                std::io::Error::last_os_error()
            ));
        }
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(format!(
                "the enrollment path {} cannot be verified as protected: {} carries a \
                 reparse point",
                path.displayed(),
                step.displayed()
            ));
        }
        Ok(())
    }

    fn elevated() -> Result<bool, String> {
        use windows_sys::Win32::Foundation::HANDLE;
        // SAFETY: GetCurrentProcess returns the current pseudo-handle.
        let process = unsafe { GetCurrentProcess() };
        let mut token = HANDLE::default();
        // SAFETY: `token` is a valid out-pointer for a handle.
        if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
            return Err(format!(
                "the enrollment protection check cannot open the process token: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0u32;
        // SAFETY: `elevation` is the correctly sized TokenElevation buffer
        // and `token` was just opened with TOKEN_QUERY.
        let read = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut _) as *mut core::ffi::c_void,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned,
            )
        };
        // SAFETY: the handle is no longer needed.
        unsafe { CloseHandle(token) };
        if read == 0 {
            return Err(format!(
                "the enrollment protection check cannot read the process token: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(elevation.TokenIsElevated != 0)
    }

    /// Query one path's owner, group, and DACL in one descriptor.
    fn query(path: &Path) -> Result<Descriptor, String> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut descriptor: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut owner: PSID = std::ptr::null_mut();
        // SAFETY: `wide` is a NUL-terminated path buffer; every out-pointer
        // is a valid slot for its query.
        let result = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if result != 0 || descriptor.is_null() {
            return Err(format!(
                "the enrollment path {} cannot be verified as protected: its protection \
                 facts cannot be read: {}",
                path.displayed(),
                std::io::Error::from_raw_os_error(result as i32)
            ));
        }
        Ok(Descriptor {
            handle: descriptor,
            owner,
        })
    }

    /// Render a SID as its canonical `S-1-...` string.
    fn sid_to_string(sid: PSID) -> Option<String> {
        // SAFETY: the PSID came from a successfully queried descriptor and
        // SIDs carry revision, count, authority, then `count` sub-authorities.
        let bytes = unsafe {
            if sid.is_null() {
                return None;
            }
            std::slice::from_raw_parts(sid as *const u8, 8 + 4 * 15)
        };
        if bytes[0] != 1 {
            return None;
        }
        let count = bytes[1] as usize;
        let mut authority: u64 = 0;
        for byte in &bytes[2..8] {
            authority = (authority << 8) | u64::from(*byte);
        }
        if bytes.len() < 8 + 4 * count {
            return None;
        }
        let mut text = format!("S-1-{authority}");
        for index in 0..count {
            let start = 8 + 4 * index;
            let sub = u32::from_le_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
            ]);
            text.push_str(&format!("-{sub}"));
        }
        Some(text)
    }

    /// Whether the builder's effective token is granted none of the
    /// dangerous rights on `path`: `AccessCheck` evaluates the process
    /// token, duplicated for impersonation, against `path`'s own owner,
    /// group, and DACL - not against a name or a single ACE, so any allow
    /// rule that reaches the builder's real groups and rights counts. A
    /// file must deny write and append, delete, ownership change, and
    /// permission change; a directory must also deny creating entries and
    /// deleting children, because a writable parent lets the builder
    /// replace the file it contains.
    fn builder_cannot_write(path: &Path, descriptor: &Descriptor) -> Result<bool, String> {
        let unverified = |detail: String| {
            format!(
                "the enrollment path {} cannot be verified as protected: the builder token's \
                 access cannot be checked: {}",
                path.displayed(),
                detail
            )
        };
        // SAFETY: every handle this block opens is stored in a guard that
        // closes it exactly once on drop, and every out-pointer names a
        // live local.
        unsafe {
            let mut process_token = core::ptr::null_mut();
            if OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_DUPLICATE | TOKEN_QUERY,
                &mut process_token,
            ) == 0
            {
                return Err(unverified(std::io::Error::last_os_error().to_string()));
            }
            let _close_process = HandleGuard(process_token);
            let mut token = core::ptr::null_mut();
            if DuplicateTokenEx(
                process_token,
                TOKEN_IMPERSONATE | TOKEN_QUERY,
                core::ptr::null(),
                SecurityImpersonation,
                TokenImpersonation,
                &mut token,
            ) == 0
            {
                return Err(unverified(std::io::Error::last_os_error().to_string()));
            }
            let _close_token = HandleGuard(token);
            let mapping = GENERIC_MAPPING {
                GenericRead: FILE_GENERIC_READ,
                GenericWrite: FILE_GENERIC_WRITE,
                GenericExecute: FILE_GENERIC_EXECUTE,
                GenericAll: FILE_ALL_ACCESS,
            };
            let mut desired = MAXIMUM_ALLOWED;
            MapGenericMask(&mut desired, &mapping);
            let mut privileges = [0u8; 1024];
            let mut privilege_size = u32::try_from(privileges.len()).unwrap_or(u32::MAX);
            let mut granted_access = 0u32;
            let mut granted = 0i32;
            if AccessCheck(
                descriptor.handle as PSECURITY_DESCRIPTOR,
                token,
                desired,
                &mapping,
                privileges.as_mut_ptr().cast::<PRIVILEGE_SET>(),
                &mut privilege_size,
                &mut granted_access,
                &mut granted,
            ) == 0
            {
                return Err(unverified(std::io::Error::last_os_error().to_string()));
            }
            MapGenericMask(&mut granted_access, &mapping);
            let dangerous = if path.is_dir() {
                FILE_ADD_FILE
                    | FILE_ADD_SUBDIRECTORY
                    | FILE_DELETE_CHILD
                    | DELETE
                    | FILE_WRITE_DAC
                    | FILE_WRITE_OWNER
            } else {
                FILE_WRITE_DATA | FILE_APPEND_DATA | DELETE | FILE_WRITE_DAC | FILE_WRITE_OWNER
            };
            Ok(granted_access & dangerous == 0)
        }
    }

    /// Closes one opened handle exactly once.
    struct HandleGuard(*mut core::ffi::c_void);

    impl Drop for HandleGuard {
        fn drop(&mut self) {
            // SAFETY: the handle was opened by the guard's constructor and
            // is closed exactly once, here.
            unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(not(windows))]
mod unix {
    use std::path::Path;

    use crate::root::Displayed;

    pub(crate) fn require_protected(
        path: &Path,
        actor: &str,
    ) -> Result<(), super::ProtectionRefusal> {
        use std::os::unix::fs::MetadataExt;

        let builder_euid = builder_euid();
        // The file and every ancestor directory, the file itself first:
        // each must be root-owned, carry no group or other write bits, and
        // never be owned by the builder actor itself.
        let mut current = Some(path.to_path_buf());
        while let Some(step) = current {
            let metadata = std::fs::symlink_metadata(&step).map_err(|error| {
                super::ProtectionRefusal::Unverifiable(format!(
                    "the enrollment path {} cannot be verified as protected: {error}",
                    path.displayed()
                ))
            })?;
            if metadata.uid() == builder_euid {
                return Err(super::ProtectionRefusal::NotProtected);
            }
            if metadata.uid() != 0 {
                return Err(super::ProtectionRefusal::NotProtected);
            }
            if metadata.mode() & 0o022 != 0 {
                return Err(super::ProtectionRefusal::NotProtected);
            }
            current = step.parent().map(std::path::PathBuf::from);
        }
        Ok(())
    }

    /// The process's effective uid, read directly from libc - never from
    /// an environment variable a builder can forge.
    fn builder_euid() -> u32 {
        extern "C" {
            fn geteuid() -> u32;
        }
        // SAFETY: `geteuid` is async-signal-safe and takes no arguments.
        unsafe { geteuid() }
    }
}
