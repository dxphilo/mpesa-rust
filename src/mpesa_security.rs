use crate::client::MpesaResult;
use crate::Mpesa;
use base64::encode;
use openssl::rsa::Padding;
use openssl::x509::X509;

/// Trait responsible for implementation of security configs for Mpesa
pub trait MpesaSecurity {
    /// Generates security credentials
    /// M-Pesa Core authenticates a transaction by decrypting the security credentials.
    /// Security credentials are generated by encrypting the base64 encoded initiator password with M-Pesa’s public key, a X509 certificate.
    /// Returns base64 encoded string.
    ///
    /// # Error
    /// Returns `EncryptionError` variant of `MpesaError`
    fn gen_security_credentials(&self) -> MpesaResult<String>;
}

impl MpesaSecurity for Mpesa {
    fn gen_security_credentials(&self) -> MpesaResult<String> {
        let pem = self.environment().get_certificate().as_bytes();
        let cert = X509::from_pem(pem)?;
        // getting the public and rsa keys
        let pub_key = cert.public_key()?;
        let rsa_key = pub_key.rsa()?;
        // configuring the buffer
        let buf_len = pub_key.size();
        let mut buffer = vec![0; buf_len];

        rsa_key.public_encrypt(self.initiator_password(), &mut buffer, Padding::PKCS1)?;
        Ok(encode(buffer))
    }
}
