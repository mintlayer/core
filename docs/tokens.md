# Mintlayer Tokens

This document describes the structure of transactions involving tokens on Mintlayer. Currently, two types of tokens are supported:

1. *MLS-01 tokens*: MLS-01 is the "basic" Mintlayer token standard, analogous to, say, ERC-20 tokens on Ethereum.

2. NFTs

A transaction involving transaction output carries a (possibly empty) `data` field specifying the purpose of the transaction.

The data field can be any of the following:

- Transfer of MLS-01 tokens or NFTs
- Token issuance
- Token burning
- NFT creation

## Transferring Tokens

To send MLT we use the MLT token ID, which is equal to 0. If the token ID is equal to the ID of an MLS-01 token, then the amount of the token is transferred to the recipient. The transaction fee is taken only in MLT. If the token ID is equal to the ID of any NFT, then the data of this NFT is transferred to the recipient without changing the creator field. The UTXO model itself allows to determine the owner of the NFT.

```rust
TxData {
    TokenTransferV1 {
        token_id: TokenID,
        amount: Value,
    }
}
```

## Issuing Tokens

To issue a new token, we specify the data for creating the token in the transaction output's `data` field:

 ```rust
TxData {
    TokenIssuanceV1 {
        token_ticker: Vec<u8>,
	amount_to_issue: Value,
        // Should not be more than 18
        number_of_decimals: u8,
	metadata_URI: Vec<u8>,
    }
}
 ```

Here, `token_ticker` is a short name given to the token (up to 5 chararcters long).

The `metatdata_URI` is a web link to a JSON file where we can store additional information about the token

The _token ID_ is defined as a hash of the _first input_ of the issuance transaction.


### Burning Tokens

The input for a token-burning transaction should be a UTXO containing tokens. In the output, the data field should contain the _TokenBurn_ variant. If the `amount_to_burn` in the output is less than the amount in the input, then there should exist at least one output for returning the difference. In this way, any existing number of tokens can be burned.

```rust
TxData {
        TokenBurnV1{
            token_id: TokenID,
            amount_to_burn: Value,
        }
    }
```
### NFT 
**TODO**

## Wallet
**TODO**

## Issuing and Transferring Tokens

```rust
/* Transfer and Issuance in one Tx */

// Alice issues 1_000_000_000 MLS-01 tokens, and sends them to Karl
let (utxo0, input0) = tx_input_gen_no_signature();
let tx = Transaction {
    inputs: vec![input0],
    outputs: vec![
        TransactionOutput::new_pubkey(
            ALICE_GENESIS_BALANCE - crate::tokens::Mlt(1000).to_munit(),
            H256::from(alice_pub_key),
        ),
        TransactionOutput::new_p2pk_with_data(
            crate::tokens::Mlt(1000).to_munit(),
            H256::from(karl_pub_key),
            OutputData::TokenIssuanceV1 {
                token_ticker: "BensT".as_bytes().to_vec(),
                amount_to_issue: 1_000_000_000,
                number_of_decimals: 2,
                metadata_uri: "mintlayer.org".as_bytes().to_vec(),
            },
        ),
    ],
    time_lock: Default::default(),
}
.sign_unchecked(&[utxo0.clone()], 0, &alice_pub_key);

assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));

let first_issuance_token_id = TokenId::new(&tx.inputs[0]);

// The newly issued token is represented by the TransactionOutput at index 1
// "Outoint" here refers to the hash of the TransactionOutput struct.
let token_utxo_hash = tx.outpoint(1);
let token_utxo = tx.outputs[1].clone();


// Let's send alice 300_000_000 and the rest back to Karl, andalso create another token, "KarlToken"
let tx = Transaction {
    inputs: vec![TransactionInput::new_empty(token_utxo_hash)],
    outputs: vec![
        TransactionOutput::new_p2pk_with_data(
            0,
            H256::from(alice_pub_key),
            OutputData::TokenTransferV1 {
                token_id: first_issuance_token_id.clone(),
                amount: 300_000_000,
            },
        ),
        TransactionOutput::new_p2pk_with_data(
            0,
            H256::from(karl_pub_key),
            OutputData::TokenTransferV1 {
                token_id: first_issuance_token_id.clone(),
                amount: 700_000_000,
            },
        ),
        TransactionOutput::new_p2pk_with_data(
            0,
           H256::from(karl_pub_key),
            OutputData::TokenIssuanceV1 {
                token_ticker: "KarlToken".as_bytes().to_vec(),
                amount_to_issue: 5_000_000_000,
                // Should be not more than 18 numbers
                number_of_decimals: 12,
                metadata_uri: "mintlayer.org".as_bytes().to_vec(),
            },
        ),
    ],
    time_lock: Default::default(),
}
.sign_unchecked(&[token_utxo.clone()], 0, &karl_pub_key);

assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));

let alice_transfer_utxo_hash = tx.outpoint(0);
let karl_transfer_utxo_hash = tx.outpoint(1);
let karl_issuance_utxo_hash = tx.outpoint(2);

assert!(!UtxoStore::<Test>::contains_key(H256::from(
    token_utxo_hash
)));
assert!(UtxoStore::<Test>::contains_key(alice_transfer_utxo_hash));
assert!(UtxoStore::<Test>::contains_key(karl_transfer_utxo_hash));
assert!(UtxoStore::<Test>::contains_key(karl_issuance_utxo_hash));

// Let's check token transfer
UtxoStore::<Test>::get(alice_transfer_utxo_hash)
    .unwrap()
    .data
    .map(|x| match x {
        OutputData::TokenTransferV1 { token_id, amount } => {
            assert_eq!(token_id, first_issuance_token_id);
            assert_eq!(amount, 300_000_000);
        }
        _ => {
            panic!("corrupted data");
        }
    })
    .unwrap();

UtxoStore::<Test>::get(karl_transfer_utxo_hash)
    .unwrap()
    .data
    .map(|x| match x {
        OutputData::TokenTransferV1 { token_id, amount } => {
            assert_eq!(token_id, first_issuance_token_id);
            assert_eq!(amount, 700_000_000);
        }
        _ => {
            panic!("corrupted data");
        }
    })
    .unwrap();

// Let's check token issuance
UtxoStore::<Test>::get(karl_issuance_utxo_hash)
    .unwrap()
    .data
    .map(|x| match x {
        OutputData::TokenIssuanceV1 {
            token_ticker,
            amount_to_issue,
            number_of_decimals,
            metadata_uri,
        } => {
            assert_eq!(token_ticker, "Token".as_bytes().to_vec());
            assert_eq!(amount_to_issue, 5_000_000_000);
            assert_eq!(number_of_decimals, 12);
            assert_eq!(metadata_uri, "mintlayer.org".as_bytes().to_vec());
        }
        _ => {
            panic!("corrupted data");
        }
    })
    .unwrap();

``` 
