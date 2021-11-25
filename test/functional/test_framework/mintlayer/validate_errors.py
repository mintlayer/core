# VE = Validate Error, 0 = no error
VE_NO_ERROR = 0
# no inputs
VE_NO_INPUTS = 1
# no outputs
VE_NO_OUTPUTS = 2
# too many inputs
VE_TOO_MANY_INPUTS = 3
# too many outputs
VE_TOO_MANY_OUTPUTS = 4
# each input should be used only once
VE_MULTIPLE_USED_INPUT = 5
# each output should be used once
VE_MULTIPLE_USED_OUTPUT = 6
# Time lock restrictions not satisfied
VE_TIME_LOCK_RESTRICTIONS_NOT_SATISFIED = 7
# Lock hash does not match
VE_LOCK_HASH_DOES_NOT_MATCH = 8
# missing inputs
VE_MISSING_INPUTS = 9
# token has never been issued
VE_TOKEN_DID_NOT_ISSUED = 10
# token ticker has none ascii characters
VE_TOKEN_TICKER_HAS_NONE_ASCII_CHAR = 11
# metadata uri has none ascii characters
VE_TOKEN_METADATAURI_HAS_NONE_ASCII_CHAR = 12
# token ticker is too long
VE_TOKEN_TICKER_TOO_LONG = 13
# token ticker can't be empty
VE_TOKEN_TICKER_CANT_BE_EMPTY = 14
# token metadata uri is too long
VE_TOKEN_METADATAURI_TOO_LONG = 15
# output value must be nonzero
VE_OUTPUT_VALUE_MUST_BE_NONEZERO = 16
# too many decimals
VE_TOO_MANY_DECIMALS = 17
# this id can't be used for a token
VE_TOKEN_ID_CAN_NOT_BE_USED = 18
# input value overflow
VE_INPUT_VALUE_OVERFLOW = 19
# output value overflow
VE_OUTPUT_VALUE_OVERFLOW = 20
# input for the token not found
VE_TOKEN_INPUT_NOT_FOUND = 21
# no inputs for the token id
VE_NO_INPUTS_FOR_TOKEN_ID = 22
# output already exists
VE_OUTPUT_EXIST = 23
# output value must not exceed input value
VE_OUTPUT_VALUE_MUST_NOT_EXCEED_INPUT_VALUE = 24
# corrupted output data
VE_CORRUPTED_OUTPUT_DATA = 25
# too many issuance in one transaction
VE_TOO_MANY_ISSUANCES = 26
# insufficient fee
VE_INSUFFICIENT_FEE = 27
# bad signature format
VE_BAD_SIGNATURE_FORMAT = 28
# signature must be valid
VE_INVALID_SIGNATURE = 29
# cannot spend a staking utxo
VE_CANNOT_SPEND_STAKING_UTXO = 30
# reward underflow
VE_REWARD_UNDERFLOW = 31
# reward exceed allowed amount
VE_REWARD_EXCEED_ALLOWED_AMOUNT = 32
# Some staking error
EV_STAKING_ERROR = 33
# Failed to convert witness to an opcode
VE_FAILED_COVERT_WITNESS_TO_OPCODE = 34
# OP_SPEND not found
VE_OPSPEND_NOT_FOUND = 35
# script verification failed
VE_SCRIPT_VERIFICATION_FAILED = 36

# Under construction
VE_RESERVED = 255

errors = (
    (VE_NO_ERROR, "no error")
    (VE_NO_INPUTS, "no inputs")
    (VE_NO_OUTPUTS, "no outputs")
    (VE_TOO_MANY_INPUTS, "no error")
    (VE_TOO_MANY_OUTPUTS, "no error")
    (VE_MULTIPLE_USED_INPUT, "no error")
    (VE_MULTIPLE_USED_OUTPUT, "no error")
    (VE_TIME_LOCK_RESTRICTIONS_NOT_SATISFIED, "no error")
    (VE_LOCK_HASH_DOES_NOT_MATCH, "no error")
    (VE_MISSING_INPUTS, "no error")
    (VE_TOKEN_DID_NOT_ISSUED, "no error")
    (VE_TOKEN_TICKER_HAS_NONE_ASCII_CHAR, "no error")
    (VE_TOKEN_METADATAURI_HAS_NONE_ASCII_CHAR, "no error")
    (VE_TOKEN_TICKER_TOO_LONG, "no error")
    (VE_TOKEN_TICKER_CANT_BE_EMPTY, "no error")
    (VE_TOKEN_METADATAURI_TOO_LONG, "no error")
    (VE_OUTPUT_VALUE_MUST_BE_NONEZERO, "no error")
    (VE_TOO_MANY_DECIMALS, "no error")
    (VE_TOKEN_ID_CAN_NOT_BE_USED, "no error")
    (VE_INPUT_VALUE_OVERFLOW, "no error")
    (VE_OUTPUT_VALUE_OVERFLOW, "no error")
    (VE_TOKEN_INPUT_NOT_FOUND, "no error")
    (VE_NO_INPUTS_FOR_TOKEN_ID, "no error")
    (VE_OUTPUT_EXIST, "no error")
    (VE_OUTPUT_VALUE_MUST_NOT_EXCEED_INPUT_VALUE, "no error")
    (VE_CORRUPTED_OUTPUT_DATA, "no error")
    (VE_TOO_MANY_ISSUANCES, "no error")
    (VE_INSUFFICIENT_FEE, "no error")
    (VE_BAD_SIGNATURE_FORMAT, "no error")
    (VE_INVALID_SIGNATURE, "no error")
    (VE_CANNOT_SPEND_STAKING_UTXO, "no error")
    (VE_REWARD_UNDERFLOW, "no error")
    (VE_REWARD_EXCEED_ALLOWED_AMOUNT, "no error")
    (EV_STAKING_ERROR, "no error")
    (VE_FAILED_COVERT_WITNESS_TO_OPCODE, "no error")
    (VE_OPSPEND_NOT_FOUND, "no error")
    (VE_SCRIPT_VERIFICATION_FAILED, "no error")
)


