use aspen_trust::chain::EncryptedSecretChain;
use aspen_trust::envelope::EncryptedValue;
use aspen_trust::envelope::EnvelopeError;
use aspen_trust::protocol::GetShareRequest;
use aspen_trust::protocol::ShareResponse;
use aspen_trust::protocol::TrustRequest;
use aspen_trust::protocol::TrustResponse;
use aspen_trust::secret::Threshold;
use aspen_trust::shamir::SECRET_SIZE;
use aspen_trust::shamir::ShamirError;
use aspen_trust::shamir::Share;
use serde_json::json;

fn fixed_share() -> Share {
    Share {
        x: 7,
        y: [0x03; SECRET_SIZE],
    }
}

#[test]
fn share_bytes_are_a_stable_contract() {
    let share = fixed_share();
    let mut expected = [0x03; SECRET_SIZE + 1];
    expected[0] = 7;

    let bytes = share.to_bytes();
    assert_eq!(bytes, expected);

    let decoded = Share::from_bytes(&bytes).expect("valid share bytes");
    assert_eq!(decoded.x, share.x);
    assert_eq!(decoded.y, share.y);

    let mut zero_x = bytes;
    zero_x[0] = 0;
    assert!(matches!(Share::from_bytes(&zero_x), Err(ShamirError::ZeroShareCoordinate)));
}

#[test]
fn encrypted_value_bytes_pin_magic_version_epoch_nonce_and_payload() {
    let value = EncryptedValue {
        version: 1,
        epoch: 0x0102_0304_0506_0708,
        nonce: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ciphertext: vec![0xaa; 16],
    };

    #[rustfmt::skip]
    let expected = vec![
        0x41, 0x45, 0x4e, 0x43, // AENC magic
        0x01, // version
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // epoch, big-endian
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
        0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, // nonce
        0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa,
        0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, // ciphertext+tag
    ];

    let bytes = value.to_bytes();
    assert_eq!(bytes, expected);
    assert_eq!(EncryptedValue::from_bytes(&bytes).expect("golden bytes decode"), value);

    let mut bad_version = bytes.clone();
    bad_version[4] = 2;
    assert!(matches!(
        EncryptedValue::from_bytes(&bad_version),
        Err(EnvelopeError::UnsupportedVersion { version: 2 })
    ));

    assert!(matches!(EncryptedValue::from_bytes(&bytes[..24]), Err(EnvelopeError::TooShort { len: 24 })));
}

#[test]
fn trust_protocol_postcard_bytes_pin_enum_order() {
    let get_share = TrustRequest::GetShare(GetShareRequest { epoch: 42 });
    assert_eq!(postcard::to_allocvec(&get_share).expect("serialize get share"), vec![0, 42]);
    assert_eq!(postcard::from_bytes::<TrustRequest>(&[0, 42]).expect("decode get share"), get_share);

    let expunged = TrustRequest::Expunged { epoch: 7 };
    assert_eq!(postcard::to_allocvec(&expunged).expect("serialize expunged"), vec![1, 7]);
    assert_eq!(postcard::from_bytes::<TrustRequest>(&[1, 7]).expect("decode expunged"), expunged);

    let response = TrustResponse::Expunged { epoch: 5 };
    assert_eq!(postcard::to_allocvec(&response).expect("serialize response"), vec![1, 5]);
    assert_eq!(postcard::from_bytes::<TrustResponse>(&[1, 5]).expect("decode response"), response);

    let share_response = TrustResponse::Share(ShareResponse {
        epoch: 11,
        current_epoch: 11,
        share: fixed_share(),
    });
    let mut expected = vec![0, 11, 11, 7];
    expected.extend_from_slice(&[0x03; SECRET_SIZE]);
    let bytes = postcard::to_allocvec(&share_response).expect("serialize share response");
    assert_eq!(bytes, expected);
    assert_eq!(postcard::from_bytes::<TrustResponse>(&bytes).expect("decode share response"), share_response);
}

#[test]
fn trust_json_contracts_cover_scalar_and_chain_state() {
    let threshold = Threshold::new(3).expect("nonzero threshold");
    assert_eq!(serde_json::to_string(&threshold).expect("serialize threshold"), "3");
    assert_eq!(serde_json::from_str::<Threshold>("3").expect("deserialize threshold").value(), 3);

    let chain = EncryptedSecretChain {
        salt: [1; 32],
        data: vec![2, 3, 4],
        epoch: 9,
        prior_count: 2,
    };
    let expected = json!({
        "salt": [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        "data": [2, 3, 4],
        "epoch": 9,
        "prior_count": 2,
    });
    let value = serde_json::to_value(&chain).expect("serialize chain");
    assert_eq!(value, expected);
    assert_eq!(serde_json::from_value::<EncryptedSecretChain>(value).expect("deserialize chain"), chain);
}
