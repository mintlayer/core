# pallet-utxo
Utxo support, based on [Substrate's workshop](https://github.com/substrate-developer-hub/utxo-workshop).

This is only the pallet; no _node_ and _runtime_ implementation.

To run the test cases, just run command `cargo test`.


### How to test in polkadot.js.org app
1. After running the core, declare the custom datatypes. GO to **Settings** > **Developer** tab and paste in the ff. JSON and then save:
```json
{
  "Value": "u128",
  "Destination": {
    "_enum": {
      "Pubkey": "Pubkey",
      "CreatePP": "DestinationCreatePP",
      "CallPP": "DestinationCallPP",
      "ScriptHash": "H256"
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
  "Pubkey": {
    "_enum": {
      "Schnorr": "Public"
    }
  },
  "Public": "H256"
}
```
2. To confirm that Alice already has UTXO at genesis, go to **Developer** > **Chain state** > **Storage**.  
For _selected state query_, choose `utxo`, and `utxoStore(H256): Option<TransactionOutput>` beside it.  
The _Option<H256>_ input box should be empty by disabling the **include option**.
Click the **+** button on the right. It should show:
```json
{
  value: 40,000,000,000,000,000,000,
  pub_key: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d,
  header: 0
}
```
3. Let's spend 50 of Alice's utxo to Bob. Go to **Developer** > **Extrinsics**.  
Choose `utxo` for _submit the following extrinsic_ dropdown. Input the ff. parameters (and then submit transaction):  
    * outpoint: `0x549bd1814a1e714aa4cd96f53fed26676021741d68e48a4c214d02aa13571304`
    * lock: `0x` (empty byte string)
    * witness (signature): `0x7860d0e15cb0dbc98c713857b334aa0fbe1c11cb7daeca09ec4b87928c9dbb34e78aea9cb5818b601c2088751477c0d3b90cd28fffc50c51a39791b8f5d3da83`
    * value: `50`
    * destination: Pubkey: `0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48`
3. Let's spend 50 of Alice's utxo to Bob. Go to **Developer** > **Extrinsics**.
   Choose `utxo` for _submit the following extrinsic_ dropdown.
   Input the following parameters (and then submit transaction):
    * outpoint: `0xfa1d0b34f8950f771881cd7a1601cc8817376ed5b18c6d528cc1ccac863482cc`
    * lock: `0x` (empty byte string)
    * witness (signature): `0x3c2f4d9264285ce26317e3aaec119db3efe46cbc7e6304d6015fced79d0d342dd9308e9fa07cbc00731513bcfc410c062c2b46083e6d1dae4ba29a95250c2f83`
    * value: `50`
    * destination: Pubkey: Schnorr: `0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48`
4. Wait for the upper right corner to change from 
```
utxo.spend
ready
```
into
```
utxo.spend
inblock
```
```
system.ExtrinsicSuccess
utxo.TransactionSuccess
extrinsic event
```

4. To verify, go back to **Developer** > **Chain state** > **Storage**, `utxo` and `utxoStore(H256): Option<TransactionOutput>`.  
Make sure the _Option<H256>_ input box is still empty, then click the **+** button. It should now show:
```json
[
  [
    [
      0x2699481f13b275dcc4e384fb513ba5472bd94d5ef288ffa5eaac9b95508d836d
    ],
    {
      value: 3,106,511,852,580,896,718,
      pub_key: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d,
      header: 0
    }
  ],
  [
    [
      0xdd22d722dade7f07b0becd3585cac0cdd17c62959229dc8d83d64b05633a60bc
    ],
    {
      value: 50,
      pub_key: 0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48,
      header: 0
    }
  ]
]
```



### How to run the benchmark in [mintlayer-node](https://github.com/mintlayer/mintlayer-node):
1. Insert this pallet-utxo crate in [pallets directory](https://github.com/mintlayer/mintlayer-node/tree/master/pallets).  

2. At runtime's [Cargo.toml](https://github.com/mintlayer/mintlayer-node/blob/master/runtime/Cargo.toml):  
  2.1. add to local dependencies:
   ```toml
   pallet-utxo = { default-features = false, path = "../pallets/utxo" }
   ```  
   2.2. add to __runtime-benchmarks__ features: 
   ```toml
   'pallet-utxo/runtime-benchmarks'
   ```  
   2.3. add to __std__ features: 
   ```toml
   'pallet-utxo/std'
   ```
   
3. At runtime's [lib.rs](https://github.com/mintlayer/mintlayer-node/blob/master/runtime/src/lib.rs):  
3.1. Import the following:
   ```rust
   pub use pallet_utxo;
   use sp_runtime::transaction_validity::{TransactionValidityError, InvalidTransaction};
   use sp_core::H256;
   use frame_support::traits::IsSubType;
   ```
   3.2. Add the utxo config:
    ```rust
    impl pallet_utxo::Config for Runtime {
        type Event = Event;
        type Call = Call;
        type WeightInfo = pallet_utxo::weights::WeightInfo<Runtime>;
    
        fn authorities() -> Vec<H256> {
            Aura::authorities()
                .iter()
                .map(|x| {
                    let r: &sp_core::sr25519::Public = x.as_ref();
                    r.0.into()
                })
                .collect()
       }
   }
    ```
   3.3. Add into `construct_runtime!` this line: 
   ```rust
   Utxo: pallet_utxo::{Pallet, Call, Config<T>, Storage, Event<T>},
   ```
   3.4. inside `fn validate_transaction()`, add this code before the `Executive::validate_transaction(source, tx)` line:
   ```rust
   if let Some(pallet_utxo::Call::spend(ref tx)) = 
        IsSubType::<pallet_utxo::Call::<Runtime>>::is_sub_type(&tx.function) {
            match pallet_utxo::validate_transaction::<Runtime>(&tx) {
                Ok(valid_tx) => { return Ok(valid_tx); }
                Err(_) => {
                    return Err(TransactionValidityError::Invalid(InvalidTransaction::Custom(1)));
                }
            }
        }
   ```
   3.5. In the function `fn dispatch_benchmark()`, add another line: 
   ```rust
   add_benchmark!(params, batches, pallet_utxo, Utxo);
   ```  
4. In node's [chain_spec.rs](https://github.com/mintlayer/mintlayer-node/blob/master/node/src/chain_spec.rs):  
4.1. Import the ff:
   ```rust 
   use node_template_runtime::{UtxoConfig, pallet_utxo};
   use sp_core:H256;
   ```
   4.2. add one more param on function `testnet_genesis()`: 
   ```rust
   endowed_utxos: Vec<sr25519::Public>
   ```
   4.3. inside function `testnet_genesis()`, create the genesis utxo:
    ```rust
    let genesis:Vec<pallet_utxo::TransactionOutput> = endowed_utxos.iter().map(|x| {
        let pub_key = H256::from_slice(x.as_slice());
        let tx_output = pallet_utxo::TransactionOutput::new(
            100 as pallet_utxo::Value,
            pub_key
        );
    
        let blake_hash = BlakeTwo256::hash_of(&tx_output);
      
        tx_output
    }).collect();
    ```
   4.4. Still inside `testnet_genesis()` function, add to the `GenesisConfig`:
    ```rust
    pallet_utxo: UtxoConfig {
                genesis_utxos: genesis,
                _marker: Default::default()
            }
    ```
   4.5. Inside both `fn development_config()` and `fn local_testnet_config()`, add the missing param of `testnet_genesis()`
   for the __endowed_utxos__:
   ```rust
   vec![
        get_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob")
   ]
   ```
5. On the terminal, move to the node directory and run 
   ```bash 
   cargo b --release --features runtime-benchmarks
   ```
6. Go back to the workspace directory `$> cd ..` and run:
   ```bash
    RUST_LOG=runtime=debug 
    target/release/node-template benchmark 
    --chain dev 
    --execution=wasm 
    --wasm-execution=compiled 
    --pallet pallet_utxo 
    --extrinsic runtime_spend 
    --steps 20 
    --repeat 10 
    --output . 
    --raw
   ```

   
