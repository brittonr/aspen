let Contract = import "../consumer-receipt-contract.ncl" in
let Valid = import "../../../tests/fixtures/executable-extent/molten-executable-extent-consumer-receipt.valid.json" in
(Valid & { ambient_authority = true }) | Contract.ConsumerReceipt
