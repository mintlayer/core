{
  "Value": "u128",
  "Destination": {
    "_enum": {
      "Pubkey": "Public",
      "CreatePP": "DestinationCreatePP",
      "CallPP": "DestinationCallPP"
    }
  },
  "DestinationCreatePP": {
    "code": "Vec<u8>",
    "data": "Vec<u8>"
  },
  "DestinationCallPP": {
    "dest_account": "AccountId",
    "input_data": "Vec<u8>"
  },
  "TransactionInput": {
    "outpoint": "Hash",
    "lock": "Vec<u8>",
    "witness": "Vec<u8>"
  },
  "TransactionOutput": {
    "value": "Value",
    "header": "TXOutputHeader",
    "destination": "Destination"
  },
  "TransactionOutputFor": "TransactionOutput",
  "Transaction": {
    "inputs": "Vec<TransactionInput>",
    "outputs": "Vec<TransactionOutput>"
  },
  "TransactionFor": "Transaction",
  "Address": "MultiAddress",
  "LookupSource": "MultiAddress",
  "TXOutputHeader": "u16",
  "Difficulty": "U256",
  "DifficultyAndTimestamp": {
    "difficulty": "Difficulty",
    "timestamp": "Moment"
  },
  "Public": "H256",
  "String": "Vec<u8>",
  "TokenID": "u64",
  "TokenInstance": {
    "id": "u64",
    "name": "String",
    "ticker": "String",
    "supply": "u128"
  },
  "TokenListData": "Vec<TokenInstance>"
}