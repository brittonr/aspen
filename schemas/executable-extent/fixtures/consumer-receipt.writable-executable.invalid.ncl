let Contract = import "../consumer-receipt-contract.ncl" in
let Valid = import "../../../tests/fixtures/executable-extent/molten-executable-extent-consumer-receipt.valid.json" in
(
  Valid
  & {
    mappings
      | force
      = [(std.array.at 0 Valid.mappings) & { mapped_state | force = "writable-and-executable" }],
  }
) | Contract.ConsumerReceipt
