use core::fmt;

const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldPromotionReferenceError {
    UnsupportedAlgorithm,
    WrongDigestLength,
    InvalidDigestSpelling,
}

macro_rules! promotion_reference {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, WorldPromotionReferenceError> {
                let value = value.into();
                validate_reference(&value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

promotion_reference!(WorldPromotionPlanRef);
promotion_reference!(WorldPromotionOperationRef);
promotion_reference!(WorldEffectIntentRef);
promotion_reference!(WorldSemanticIntentRef);
promotion_reference!(WorldReleaseReservationRef);
promotion_reference!(WorldReleaseAttemptRef);
promotion_reference!(WorldReleaseObservationRef);
promotion_reference!(WorldPromotionAuthorityRef);
promotion_reference!(WorldPromotionHandlerRef);
promotion_reference!(WorldPromotionAdapterRef);

fn validate_reference(value: &str) -> Result<(), WorldPromotionReferenceError> {
    let Some(hex) = value.strip_prefix(BLAKE3_PREFIX) else {
        return Err(WorldPromotionReferenceError::UnsupportedAlgorithm);
    };
    if hex.len() != BLAKE3_HEX_LENGTH {
        return Err(WorldPromotionReferenceError::WrongDigestLength);
    }
    if !hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(WorldPromotionReferenceError::InvalidDigestSpelling);
    }
    Ok(())
}
