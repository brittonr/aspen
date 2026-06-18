#[path = "receipts/command.rs"]
mod command;
#[path = "receipts/io.rs"]
mod io;
#[path = "receipts/keyring.rs"]
mod keyring;
#[path = "receipts/keys.rs"]
mod keys;
#[path = "receipts/operator.rs"]
mod operator;
#[path = "receipts/signing.rs"]
mod signing;

pub(crate) type ReceiptsCommand = command::Top;
pub(crate) type ReceiptCommand = command::Test;
type ReceiptKeyCommand = command::Key;
pub(crate) type SignedReceiptKeyring = keyring::Set;

pub(crate) fn run_receipts_command(command: ReceiptsCommand) -> molten::error::Result<()> {
    operator::run(command)
}

pub(crate) fn run_receipt_command(command: ReceiptCommand) -> molten::error::Result<()> {
    match command {
        ReceiptCommand::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        } => signing::run_test_sign(signing::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        }),
        ReceiptCommand::Verify {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        } => signing::run_test_verify(signing::Verify {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        }),
    }
}

pub(crate) fn ensure_keyring_selector_has_ledger(
    ledger: Option<&std::path::Path>,
    key_ref: Option<&str>,
    key_id: Option<&str>,
) -> molten::error::Result<()> {
    keyring::ensure_selector_has_ledger(ledger, key_ref, key_id)
}

pub(crate) fn load_signed_receipt_keyring(ledger: &std::path::Path) -> molten::error::Result<SignedReceiptKeyring> {
    keyring::load(ledger)
}
