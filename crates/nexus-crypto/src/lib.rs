use std::fmt;

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

const SESSION_KEY_INFO: &[u8] = b"NEXUS/0.2/session-key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EphemeralKeypair {
    private_key: [u8; 32],
    pub public_key: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedPayload {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityKeypair {
    pub name: String,
    pub public_key_hex: String,
    pub private_key_hex: String,
    pub node_id: String,
}

impl fmt::Display for IdentityKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.node_id)
    }
}

pub fn generate_identity(name: &str) -> IdentityKeypair {
    let mut csprng = rand::rngs::OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let public_key = signing_key.verifying_key();
    let private_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(public_key.to_bytes());
    let node_id = derive_node_id(&public_hex);

    IdentityKeypair {
        name: name.to_string(),
        public_key_hex: public_hex,
        private_key_hex: private_hex,
        node_id,
    }
}

pub fn derive_node_id(public_key_hex: &str) -> String {
    let bytes = hex::decode(public_key_hex).unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    format!("nxid:{}", hex::encode(&digest[..12]))
}

pub fn sign_payload(private_key_hex: &str, payload: &[u8]) -> Vec<u8> {
    let bytes = hex::decode(private_key_hex).unwrap_or_default();
    let secret = SigningKey::from_bytes(&bytes.as_slice().try_into().unwrap_or([0u8; 32]));
    secret.sign(payload).to_bytes().to_vec()
}

pub fn verify_signature(public_key_hex: &str, payload: &[u8], signature: &[u8]) -> bool {
    let public_bytes = hex::decode(public_key_hex).unwrap_or_default();
    let key = VerifyingKey::from_bytes(&public_bytes.as_slice().try_into().unwrap_or([0u8; 32]));
    let sig = ed25519_dalek::Signature::from_slice(signature);
    match (key, sig) {
        (Ok(key), Ok(signature)) => key.verify(payload, &signature).is_ok(),
        _ => false,
    }
}

pub fn generate_ephemeral_keypair() -> EphemeralKeypair {
    let mut private_key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut private_key);
    let secret = StaticSecret::from(private_key);
    EphemeralKeypair {
        private_key,
        public_key: PublicKey::from(&secret).to_bytes(),
    }
}

pub fn derive_ephemeral_session_key(
    local: &EphemeralKeypair,
    remote_public_key: &[u8; 32],
    transcript: &[u8],
) -> [u8; 32] {
    let secret = StaticSecret::from(local.private_key);
    let remote = PublicKey::from(*remote_public_key);
    let shared_secret = secret.diffie_hellman(&remote);
    let transcript_hash = Sha256::digest(transcript);
    let hkdf = Hkdf::<Sha256>::new(Some(&transcript_hash), shared_secret.as_bytes());
    let mut session_key = [0u8; 32];
    hkdf.expand(SESSION_KEY_INFO, &mut session_key)
        .expect("NEXUS session key length is valid");
    session_key
}

pub fn derive_session_id(transcript: &[u8]) -> String {
    let digest = Sha256::digest(transcript);
    format!("sess:{}", hex::encode(&digest[..16]))
}

pub fn random_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn encrypt_aead(
    secret: &[u8; 32],
    plaintext: &[u8],
    associated_data: &[u8],
) -> EncryptedPayload {
    let key = chacha20poly1305::Key::from_slice(secret);
    let cipher = ChaCha20Poly1305::new(key);
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .expect("NEXUS AEAD encryption cannot fail with valid inputs");
    EncryptedPayload {
        nonce: nonce_bytes,
        ciphertext,
    }
}

pub fn decrypt_aead(
    secret: &[u8; 32],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    associated_data: &[u8],
) -> Result<Vec<u8>, String> {
    let key = chacha20poly1305::Key::from_slice(secret);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(
            nonce,
            chacha20poly1305::aead::Payload {
                msg: ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|_| "invalid AEAD tag or associated data".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        decrypt_aead, derive_ephemeral_session_key, derive_node_id, encrypt_aead,
        generate_ephemeral_keypair, generate_identity, sign_payload, verify_signature,
    };

    #[test]
    fn identity_generation_creates_node_id() {
        let identity = generate_identity("alice");
        assert!(identity.node_id.starts_with("nxid:"));
        assert_eq!(identity.node_id, derive_node_id(&identity.public_key_hex));
    }

    #[test]
    fn signature_verification_round_trip() {
        let identity = generate_identity("alice");
        let payload = b"hello from alice";
        let signature = sign_payload(&identity.private_key_hex, payload);
        assert!(verify_signature(
            &identity.public_key_hex,
            payload,
            &signature
        ));
    }

    #[test]
    fn ephemeral_keys_derive_the_same_session_key() {
        let alice = generate_ephemeral_keypair();
        let bob = generate_ephemeral_keypair();
        let transcript = b"alice-bob-transcript";
        let alice_key = derive_ephemeral_session_key(&alice, &bob.public_key, transcript);
        let bob_key = derive_ephemeral_session_key(&bob, &alice.public_key, transcript);
        assert_eq!(alice_key, bob_key);
    }

    #[test]
    fn aead_authenticates_metadata_and_uses_fresh_nonces() {
        let alice = generate_ephemeral_keypair();
        let bob = generate_ephemeral_keypair();
        let key = derive_ephemeral_session_key(&alice, &bob.public_key, b"transcript");
        let first = encrypt_aead(&key, b"hello", b"session=one;sequence=1");
        let second = encrypt_aead(&key, b"hello", b"session=one;sequence=1");
        assert_ne!(first.nonce, second.nonce);
        assert_eq!(
            decrypt_aead(
                &key,
                &first.nonce,
                &first.ciphertext,
                b"session=one;sequence=1"
            )
            .unwrap(),
            b"hello"
        );
        assert!(decrypt_aead(
            &key,
            &first.nonce,
            &first.ciphertext,
            b"session=one;sequence=2"
        )
        .is_err());
    }
}
