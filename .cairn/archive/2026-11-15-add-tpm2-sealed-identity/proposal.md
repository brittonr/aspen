# Add TPM2 sealed transport identity

Issue: brittonr/aspen#9. ManagedSecret has no live loader and is not a TPM implementation. Add an explicit PCR-policy-bound Tpm2Sealed backend, a retained-capability bounded tpm2-tools shell, and a disposable swtpm rail. Keep key possession separate from membership and authority. No PKCS11 signer, remote attestation, automatic provisioning or file fallback.
