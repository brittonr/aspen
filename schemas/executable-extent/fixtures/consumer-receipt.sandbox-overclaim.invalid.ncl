let Contract = import "../consumer-receipt-contract.ncl" in
let Valid = import "../../../tests/fixtures/executable-extent/molten-executable-extent-consumer-receipt.valid.json" in
(
  Valid
  & {
    non_claims
      | force
      = ["mapping proves sandbox"] @ std.array.drop 1 Valid.non_claims,
  }
) | Contract.ConsumerReceipt
