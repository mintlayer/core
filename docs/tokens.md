# Mintlayer Tokens
**TODO Do we want Rust code in this doc?**

Each transaction output must carry a data field. This field describes the purpose of the transaction.

We must highlight the following at the moment:

- Transfer Tokens or NFT
- Token Issuance
- Burning tokens
- NFT creation

All transactions must be signed on the client side. This means we cannot sign transactions on the node side.
**TODO what is the connection between these two sentences?**
In the future, there will be changes to the structures of transactions and we will be required to track them.
**TODO by structures above to we mean rust structs? Why is this interesting to the user?**

## Transfer Tokens 

**TODO sentence fragment, I don't understand...**
**TODO When do we NOT use the TxData field?**
For transfering funds to another person in a given UTXO. To send MLT we will use the MLT token ID, which is equal to 0. If the token ID is equal to the ID of the MLS-01 (**TODO what is MLS-01**) token, then the amount of token is transferred to the recipient. The commission is taken only in MLT (**TODO what is this commission**. If the token ID is equal to the ID of any NFT, then the data of this NFT is transferred to the recipient without changing the creator field. The UTXO model itself allows to determine the owner of the NFT.

```rust
TxData {
    TokenTransferV1 {
        token_id: TokenID,
        amount: Value,
    }
}
```

## Issue Tokens
When issuing a new token, we specify the data for creating a new token in the transaction input, where the `token_id` is a hash of the inputs. **TODO which inputs?**
**TODO explain remaining fields**

**TODO understand the comment**
```rust
TxData {
    TokenIssuanceV1 {
                token_id: TokenID,
                token_ticker: Vec<u8>,
                amount_to_issue: Value,
                // Should be not more than 18 numbers
                number_of_decimals: u8,
                metadata_URI: Vec<u8>,
            }
}
```

### Burn Tokens
**TODO verify - the input should be a utxo that contains tokens, the output should contain the TokenBurn arm**
A token burning - as an input is used by UTXO that contains tokens. As an output, the data field should contain the TokenBurn arm. If the amount in burning the output is less than in the input then there should exist at least one output for returning the funds change. In this case, you can burn any existing number of tokens. After this operation, you can use UTXO for the remaining amount of tokens.

```rust
TxData {
        TokenBurnV1{
            token_id: TokenID,
            amount_to_burn: Value,
        }
    }
```
### NFT 
TO DO

## Wallet

TO DO
## Issue and Transfer Tokens

**TODO who are these examples meant for?**
```rust
/* Transfer and Issuance in one Tx */

// Alice issues 1_000_000_000 MLS-01, and send them to Karl
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

let first_issuance_token_id = TokenId::new(&tx.inputs[0]);
assert_ok!(Utxo::spend(Origin::signed(H256::zero()), tx.clone()));
let token_utxo_hash = tx.outpoint(1);
let token_utxo = tx.outputs[1].clone();


// Let's send 300_000_000 and rest back and create another token
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
                token_ticker: "Token".as_bytes().to_vec(),
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
