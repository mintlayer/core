# Description

We cannot sign transactions on the node side, all transactions must be signed on the client side. In the future, there will be changes to the structures of transactions and we will be required to track them. The ability to input any input data should be covered by tests. 

## Enum TransactionOutput.data

Each transaction output must carry a `data` field. This field is the designation of the purpose of the transaction, we must highlight the following at the moment:

* Transfer Tokens or NFT
* Token Issuance
* Burning token
* NFT creation

This field must be `Enum`. All of the `TransactionOutput` structure and related types ​​must be derived `Encode` and `Decode`. This will allow SCALE Codec to correctly decode data fields. That there is no confusion due to changes in data places, we must mark each element of the type with the index `#[codec(index = NUM)]`.

And there is a requirement for testing, each version must be covered with tests that are guaranteed to break if someone changes the internal data in any version. 

```rust
    #[derive(Encode, Decode...)]
    pub struct TransactionOutput<AccountId> {
        #[codec(index = 1)]
        pub(crate) value: Value,
        #[codec(index = 2)]
        pub(crate) destination: Destination<AccountId>,
        #[codec(index = 3)]
        pub(crate) data: Option<TxData>,
    }

    #[derive(Encode, Decode...)]
    pub enum TxData {
        // TokenTransfer data to another user. If it is a token, then the token data must also be transferred to the recipient. 
        #[codec(index = 1)]
        TokenTransferV1(..),
        // A new token creation
        #[codec(index = 2)]
        TokenIssuanceV1(..),
        // Burning a token or NFT
        #[codec(index = 3)]
        TokenBurnV1(..),
        // A new NFT creation
        #[codec(index = 4)]
        NftMintV1(..),
        ...
        // We do not change the data in TokenTransferV1, we just create a new version 
        #[codec(index = 5)]
        TokenTransferV2(..),
    }
```

## Detailed description of Enum TxData fields

### TokenTransferV1
---

Transfer funds to another person in a given UTXO. To send MLT we will use `TokenID :: MLT`, which is actually zero. If the `token_id` is equal to the ID of the MLS-01 token, then the amount of token is transferred to the recipient. The commission is taken only in MLT. If the `token_id` is equal to the id of any NFT, then the data of this NFT should be transferred to the recipient without changing the creator field. The UTXO model itself allows you to determine the owner of the NFT. 

```rust
    pub enum TxData {
        TokenTransferV1{
            token_id: TokenID,
            amount: Value,
        }
        ...
    }
```

### TokenIssuanceV1
---

When issuing a new token, we specify the data for creating a new token in the transaction input, where the `token_id` is: 

```rust
    let token_id = BlakeTwo256::hash_of(&(&tx.inputs[0], &tx.inputs[0].index));
```

`token_ticker` - might be not unique. Should be limited to 5 chars. In fact, it's an ASCII string.

```rust
    pub enum TxData {
        TokenIssuanceV1{
            token_id: TokenID,
            token_ticker: Vec<u8>,
            amount_to_issue: Value,
            // Should be not more than 18 numbers
            number_of_decimals: u8,
            metadata_URI: Vec<u8>,
        }
        ...
    }
```
See the `metada_URI` format below. 

### TokenBurnV1
---

A token burning - as an input is used by UTXO that containing tokens. As an output, the data field should contain the TokenBurnV1 arm. If the amount in burning the output is less than in the input then should exist at least one output for returning the funds change. In this case, you can burn any existing number of tokens. After this operation, you can use UTXO for the remaining amount of tokens.
```rust
    type String = Vec<u8>;
    pub enum TxData {
        TokenBurnV1{
            token_id: TokenID,
            amount_to_burn: Value,
        }
        ...
    }
```

### NftMintV1
---
When minting a new NFT token, we specify the data for creating a new token in the transaction input, where the `token_id` is: 

```rust
    let token_id = BlakeTwo256::hash_of(&(&tx.inputs[0], &tx.inputs[0].index));
```

For the seek a creation UTXO, we should make a new Storage where:
* Key - token_id
* Value - hash of UTXO

It allows us to find the whole information about the NFT including `creator`, and it won't be changed. It is suitable for the MLS-01 tokens too.

The `data_hash` field is a hash of external data, which should be taken from the digital data for which the NFT is being created. This field should also not be changed when sent to a new owner.

The `metadata_URI` field can contain the name of the asset and its description, as well as an image with its data for preview.

It is also possible to add different types of hashes and owners. 

```rust    
    #[derive(Encode, Decode, ...)]
    pub enum TxData {
        MintV1{
            token_id: TokenID,
            data_hash: NftDataHash,
            metadata_URI: Vec<u8>,
        }
        ...
    }

    #[derive(Encode, Decode, ...)]
    pub enum NftDataHash {
        #[codec(index = 1)]
        Hash32([u8; 32]),
        #[codec(index = 2)]
        Raw(Vec<u8>),
        // Or any type that you want to implement
    }
```

### Error Handling 

We should use `"chain-error"` feature for the SCALE Codec. It allows us to get a more detailed description of errors. 

```rust
[dependencies.codec]
default-features = false
features = ["derive", "chain-error"]
``` 

However, this kind of error might show only place in data that didn't decode or encoded correctly. Example:

```bash
"Could not decode `TransactionOutputV1::data`:\n\tCould not decode `TxDataV1`, variant doesn't exist\n"
```
Anyway, the correctness of decoded data we should check additionally.

### Adding a new version

Adding a new version of data is in fact adding a new field to the enum, if the names match, add the version number at the end, for example:

* TokenTransferV1
* TokenTransferV2
* etc

The order of the fields is not important, but each field must be marked with a unique codec index - `# [codec (index = your index)]`. Example:

```rust
#[derive(Encode, Decode, ...)]
pub enum TxDataV2 {
    #[codec(index = 2)]
    NftMintV2 {
        id: u64,
        token_name: Vec<u8>,
        // other fields that you wish
    },
    #[codec(index = 1)]
    NftMintV1 { id: u64 },
}
```

You also need to add an appropriate test to track changes. 

Example: [check_immutability test](https://github.com/sinitcin/scale_test/blob/b95a19708c3f65a0b9499fcd19f1e081a843cc4a/src/main.rs#L124)

This test will compare against the structure template and if someone accidentally changes the data fields, the test will indicate this. 

### What happens if the old version of the node reads the new transaction format?

The transaction can not be processing. Prove: [an_old_node_read_a_new_data test](https://github.com/sinitcin/scale_test/blob/b95a19708c3f65a0b9499fcd19f1e081a843cc4a/src/main.rs#L93)

### What happens if the new version of the node reads the old transaction format?

Transaction data will be correctly read and, depending on the blockchain logic, interpreted or discarded. Example:

```rust
match data {
    TokenTransferV1(...) => pallet::transfer_v1(...),
    TokenTransferV2(...) => pallet::transfer_v2(...),
}
```

Prove: [a_new_node_read_an_old_data test](https://github.com/sinitcin/scale_test/blob/b95a19708c3f65a0b9499fcd19f1e081a843cc4a/src/main.rs#L109)

### Format of data located by reference `metadata_URI`

This is a link to a third-party server that will contain a json format similar to “ERC721 Metadata JSON Schema”: 

```json
{
    "title": "Asset Metadata",
    "properties": {
        "name": {
            "type": "string",
            "description": "Identifies the asset to which this token represents"
        },
        "description": {
            "type": "string",
            "description": "Describes the asset to which this token represents"
        },
        "image": {
            "type": "string",
            "description": "A URI pointing to a resource with mime type image/* representing the asset to which this token represents. Consider making any images at a width between 320 and 1080 pixels and aspect ratio between 1.91:1 and 4:5 inclusive."
        }
    }
}
```

This file will be used on blockchain explorers.

### Unit testing

Over here is suggested about test plan for tokens and data field:
* All tests must apply to all possible versions of the data field. 
* Also, tests should be carried out in the mode of one input - one output, as well as multiple inputs - multiple outputs. 
* Also the tests below should be applied to cases without tokens, for MLS-01, as well as for NFT. 

**General checks to be repeated for each type of token:**

  1. **Testing token creation**:
     * Creation a token with a pre-existing ID or re-creation of an already created token. 
       * The action could not be completed. The error must be handled correctly.

     * Creating a token with corrupted data 
       * Data field of zero length 
       * The data field of the maximum allowed length filled with random garbage 
       * Creation of a token with 0 issue amount 
       * Generating a token with a long URI string 

     * Creation of a token without input with MLT to pay commission 
       * Test tx where Input with token and without MLT, output has token (without MLT) 
       * Test tx where Input with token and without MLT, output has MLT (without token) 
       * Test tx where Input without token but with MLT, output has MLT and token
       * Test tx where no inputs for token 
       * Test where less MLT at the input than you need to pay the commission 
       * Test tx where Input and output have a token but with zero value
  
  2. **Testing token transfer**
     * Standard creation of a token and sending it to a chain of persons, and from them collecting the token into one UTXO and checking that the token data has not changed, has not been burned or lost in any way.
       * All data must be correct for this test.
       * The token must be sent through multiple account groups.
       * The total amount of the token must be equal to the created one.
     * Incorrect amount of token in one input and one output
       * The input contains the correct amount of token, but the output is incorrect
       * In the input, the number is incorrect, but the output is correct
       * Entry and exit with incorrect number of tokens
     * Testing UTXO for token and return funds change
     * Use in one MLT input to pay the commission and transfer the token at the same time 
     * Check possibility to cause overflow. For example, let's take a few inputs where value is a maximum possible number, and one input should have the sum of these inputs values. 

  3. **Testing the compatibility of the old version with the new one**
     * Testing the compatibility of the new version with the old new
       * Testing data encoding in a loop
       * Testing the processing of junk and random data
       * Testing the processing of fields that are written in a different order
       * Testing the immutability of old versions 
  
  4. **Testing burning tokens**
     * Trying to burn none-existing token
     * Trying to burn more token value than exist in inputs
     * Trying to burn MLT
     * Trying to burn MLS-01
     * Trying to burn NFT
     * Trying to burn token without inputs for that
     * Trying to burn existing token, but which is not in the input 

**What we shall test additionally?**
1. I can't make any limits on data fields sizes through SCALE. I'm pretty sure that Substrate checks size limits for the whole transactions because I take from framework already decoded structures. I can't see raw bytes. But I can't prove it without testing. 
2. All maths with `u128`/`i128` should be double-checked and tested to prevent overflow and underflow.
3. Functional tests - will be planned later.