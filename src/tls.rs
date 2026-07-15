pub(crate) fn install_crypto_provider() {
    // Reqwest and SQLx share this process-wide provider.
    let _ = rustls::crypto::ring::default_provider().install_default();
}
