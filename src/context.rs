use crate::{
    errors::LoadRootCertificateError, session::WolfSession, RootCertificate, Secret, WolfMethod,
};

use parking_lot::Mutex;
use std::sync::Arc;

#[allow(missing_docs)]
#[derive(Debug)]
pub struct WolfContextBuilder(WolfContext);

impl WolfContextBuilder {
    /// Invokes [`wolfSSL_CTX_new`][0]
    ///
    /// [0]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__Setup.html#function-wolfssl_ctx_new
    pub fn new(method: WolfMethod) -> Option<Self> {
        let method_fn = method.into_method_ptr();

        let ctx = unsafe { wolfssl_sys::wolfSSL_CTX_new(method_fn) };

        if !ctx.is_null() {
            Some(WolfContextBuilder(WolfContext {
                ctx: Arc::new(Mutex::new(ctx)),
                method,
            }))
        } else {
            None
        }
    }

    /// Wraps [`wolfSSL_CTX_load_verify_buffer`][0] and [`wolfSSL_CTX_load_verify_locations`][1]
    ///
    /// [0]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__CertsKeys.html#function-wolfssl_ctx_load_verify_buffer
    /// [1]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__CertsKeys.html#function-wolfssl_ctx_load_verify_locations
    pub fn with_root_certificate(
        self,
        root: RootCertificate,
    ) -> Result<Self, LoadRootCertificateError> {
        use wolfssl_sys::{
            wolfSSL_CTX_load_verify_buffer, wolfSSL_CTX_load_verify_locations,
            WOLFSSL_FILETYPE_ASN1, WOLFSSL_FILETYPE_PEM, WOLFSSL_SUCCESS,
        };

        let ctx = self.0.ctx.lock();
        let result = match root {
            RootCertificate::Asn1Buffer(buf) => unsafe {
                wolfSSL_CTX_load_verify_buffer(
                    *ctx,
                    buf.as_ptr(),
                    buf.len() as i64,
                    WOLFSSL_FILETYPE_ASN1,
                )
            },
            RootCertificate::PemBuffer(buf) => unsafe {
                wolfSSL_CTX_load_verify_buffer(
                    *ctx,
                    buf.as_ptr(),
                    buf.len() as i64,
                    WOLFSSL_FILETYPE_PEM,
                )
            },
            RootCertificate::PemFileOrDirectory(path) => {
                let is_dir = path.is_dir();
                let path =
                    std::ffi::CString::new(path.to_str().ok_or(LoadRootCertificateError::Path)?)
                        .map_err(|_| LoadRootCertificateError::Path)?;
                if is_dir {
                    unsafe {
                        wolfSSL_CTX_load_verify_locations(
                            *ctx,
                            std::ptr::null(),
                            path.as_c_str().as_ptr(),
                        )
                    }
                } else {
                    unsafe {
                        wolfSSL_CTX_load_verify_locations(
                            *ctx,
                            path.as_c_str().as_ptr(),
                            std::ptr::null(),
                        )
                    }
                }
            }
        };
        drop(ctx);

        if result == WOLFSSL_SUCCESS {
            Ok(self)
        } else {
            Err(LoadRootCertificateError::from(result))
        }
    }

    /// Wraps [`wolfSSL_CTX_set_cipher_list`][0]
    ///
    /// [0]: https://www.wolfssl.com/documentation/manuals/wolfssl/ssl_8h.html#function-wolfssl_ctx_set_cipher_list
    pub fn with_cipher_list(self, cipher_list: &str) -> Option<Self> {
        let cipher_list = std::ffi::CString::new(cipher_list).ok()?;
        let result = unsafe {
            let ctx = self.0.ctx.lock();
            wolfssl_sys::wolfSSL_CTX_set_cipher_list(*ctx, cipher_list.as_c_str().as_ptr())
        };
        if result == wolfssl_sys::WOLFSSL_SUCCESS {
            Some(self)
        } else {
            None
        }
    }

    /// Wraps [`wolfSSL_CTX_use_certificate_file`][0] and [`wolfSSL_CTX_use_certificate_buffer`][1]
    ///
    /// [0]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__CertsKeys.html#function-wolfssl_ctx_use_certificate_file
    /// [1]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__CertsKeys.html#function-wolfssl_ctx_use_certificate_buffer
    pub fn with_certificate(self, secret: Secret) -> Option<Self> {
        use wolfssl_sys::{
            wolfSSL_CTX_use_certificate_buffer, wolfSSL_CTX_use_certificate_file,
            WOLFSSL_FILETYPE_ASN1, WOLFSSL_FILETYPE_PEM, WOLFSSL_SUCCESS,
        };

        let ctx = self.0.ctx.lock();
        let result = match secret {
            Secret::Asn1Buffer(buf) => unsafe {
                wolfSSL_CTX_use_certificate_buffer(
                    *ctx,
                    buf.as_ptr(),
                    buf.len() as i64,
                    WOLFSSL_FILETYPE_ASN1,
                )
            },
            Secret::Asn1File(path) => unsafe {
                let file = std::ffi::CString::new(path.to_str()?).ok()?;
                wolfSSL_CTX_use_certificate_file(
                    *ctx,
                    file.as_c_str().as_ptr(),
                    WOLFSSL_FILETYPE_ASN1,
                )
            },
            Secret::PemBuffer(buf) => unsafe {
                wolfSSL_CTX_use_certificate_buffer(
                    *ctx,
                    buf.as_ptr(),
                    buf.len() as i64,
                    WOLFSSL_FILETYPE_PEM,
                )
            },
            Secret::PemFile(path) => unsafe {
                let file = std::ffi::CString::new(path.to_str()?).ok()?;
                wolfSSL_CTX_use_certificate_file(
                    *ctx,
                    file.as_c_str().as_ptr(),
                    WOLFSSL_FILETYPE_PEM,
                )
            },
        };
        drop(ctx);

        if result == WOLFSSL_SUCCESS {
            Some(self)
        } else {
            None
        }
    }

    /// Wraps [`wolfSSL_CTX_use_PrivateKey_file`][0] and [`wolfSSL_CTX_use_PrivateKey_buffer`][1]
    ///
    /// [0]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__CertsKeys.html#function-wolfssl_ctx_use_privatekey_file
    /// [1]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__CertsKeys.html#function-wolfssl_ctx_use_privatekey_buffer
    pub fn with_private_key(self, secret: Secret) -> Option<Self> {
        use wolfssl_sys::{
            wolfSSL_CTX_use_PrivateKey_buffer, wolfSSL_CTX_use_PrivateKey_file,
            WOLFSSL_FILETYPE_ASN1, WOLFSSL_FILETYPE_PEM, WOLFSSL_SUCCESS,
        };

        let ctx = self.0.ctx.lock();
        let result = match secret {
            Secret::Asn1Buffer(buf) => unsafe {
                wolfSSL_CTX_use_PrivateKey_buffer(
                    *ctx,
                    buf.as_ptr(),
                    buf.len() as i64,
                    WOLFSSL_FILETYPE_ASN1,
                )
            },
            Secret::Asn1File(path) => unsafe {
                let path = std::ffi::CString::new(path.to_str()?).ok()?;
                wolfSSL_CTX_use_PrivateKey_file(
                    *ctx,
                    path.as_c_str().as_ptr(),
                    WOLFSSL_FILETYPE_ASN1,
                )
            },
            Secret::PemBuffer(buf) => unsafe {
                wolfSSL_CTX_use_PrivateKey_buffer(
                    *ctx,
                    buf.as_ptr(),
                    buf.len() as i64,
                    WOLFSSL_FILETYPE_PEM,
                )
            },
            Secret::PemFile(path) => unsafe {
                let path = std::ffi::CString::new(path.to_str()?).ok()?;
                wolfSSL_CTX_use_PrivateKey_file(
                    *ctx,
                    path.as_c_str().as_ptr(),
                    WOLFSSL_FILETYPE_PEM,
                )
            },
        };
        drop(ctx);

        if result == WOLFSSL_SUCCESS {
            Some(self)
        } else {
            None
        }
    }

    /// Wraps `wolfSSL_CTX_UseSecureRenegotiation`
    ///
    /// Note that this only works on DTLS1.2
    pub fn with_secure_renegotiation(self) -> Option<Self> {
        if !matches!(
            self.0.method(),
            WolfMethod::DtlsClientV1_2 | WolfMethod::DtlsServerV1_2
        ) {
            log::warn!("Attempted to enable secure renegotiation while not on DTLS 1.2");
            return Some(self);
        }

        let result = unsafe {
            let ctx = self.0.ctx.lock();
            wolfssl_sys::wolfSSL_CTX_UseSecureRenegotiation(*ctx)
        };
        if result == wolfssl_sys::WOLFSSL_SUCCESS {
            Some(self)
        } else {
            None
        }
    }

    /// Finalizes a `WolfContext`.
    pub fn build(self) -> WolfContext {
        self.0
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct WolfContext {
    pub(crate) ctx: Arc<Mutex<*mut wolfssl_sys::WOLFSSL_CTX>>,
    method: WolfMethod,
}

impl WolfContext {
    /// Returns the [`WolfMethod`] used to initialize this
    /// [`WolfContext`].
    pub fn method(&self) -> WolfMethod {
        self.method
    }

    /// Invokes [`wolfSSL_new`][0]
    ///
    /// [0]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__Setup.html#function-wolfssl_new
    pub fn new_session(&self) -> Option<WolfSession> {
        let ctx = self.ctx.lock();
        let ptr = unsafe { wolfssl_sys::wolfSSL_new(*ctx) };
        if !ptr.is_null() {
            Some(WolfSession {
                ctx: self.clone(),
                ssl: Mutex::new(ptr),
            })
        } else {
            None
        }
    }
}

impl Drop for WolfContext {
    /// Invokes [`wolfSSL_CTX_free`][0]
    ///
    /// [0]: https://www.wolfssl.com/documentation/manuals/wolfssl/group__Setup.html#function-wolfssl_ctx_free
    fn drop(&mut self) {
        let ctx = self.ctx.lock();
        if Arc::strong_count(&self.ctx) == 1 {
            unsafe { wolfssl_sys::wolfSSL_CTX_free(*ctx) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wolf_cleanup;

    #[test]
    fn wolf_context_new() {
        WolfContextBuilder::new(WolfMethod::DtlsClient).unwrap();
        wolf_cleanup().unwrap();
    }

    #[test]
    fn wolf_context_root_certificate_buffer() {
        const CA_CERT: &[u8] = &include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/ca_cert_der_2048"
        ));

        let cert = RootCertificate::Asn1Buffer(CA_CERT);

        let _ = WolfContextBuilder::new(WolfMethod::TlsClient)
            .unwrap()
            .with_root_certificate(cert)
            .unwrap();

        wolf_cleanup().unwrap();
    }

    #[test]
    fn wolf_context_set_cipher_list() {
        let _ = WolfContextBuilder::new(WolfMethod::DtlsClient)
            .unwrap()
            // This string might need to change depending on the flags
            // we built wolfssl with.
            .with_cipher_list("TLS13-CHACHA20-POLY1305-SHA256")
            .unwrap();

        wolf_cleanup().unwrap();
    }

    #[test]
    fn wolf_context_set_certificate_buffer() {
        const SERVER_CERT: &[u8] = &include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/server_cert_der_2048"
        ));

        let cert = Secret::Asn1Buffer(SERVER_CERT);

        let _ = WolfContextBuilder::new(WolfMethod::TlsClient)
            .unwrap()
            .with_certificate(cert)
            .unwrap();

        wolf_cleanup().unwrap();
    }

    #[test]
    fn wolf_context_set_private_key_buffer() {
        const SERVER_KEY: &[u8] = &include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/server_key_der_2048"
        ));

        let key = Secret::Asn1Buffer(SERVER_KEY);

        let _ = WolfContextBuilder::new(WolfMethod::TlsClient)
            .unwrap()
            .with_private_key(key)
            .unwrap();

        wolf_cleanup().unwrap();
    }

    #[test]
    fn wolf_context_set_secure_renegotiation() {
        let _ = WolfContextBuilder::new(WolfMethod::DtlsClientV1_2)
            .unwrap()
            .with_secure_renegotiation()
            .unwrap();

        wolf_cleanup().unwrap();
    }
}
