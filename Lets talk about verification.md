**This description is still approximate and not accurate, we need to define an approach and agree on checks.**

## Draft TransactionVerifier

I suggest adding a structure that will contain: 

```rust
pub struct TransactionVerifier<'a, T: frame_system::Config> {
    // Pointer to a tx that we have to check 
    tx: &'a TransactionFor<T>,
    // All inputs, to avoid repeated search in the loop 
    all_inputs_map: BTreeMap<TokenId, TransactionOutputFor<T>>,
    // All outputs, to avoid repeated search in the loop 
    all_outputs_map: BTreeMap<TokenId, TransactionOutputFor<T>>,
    // Using TokenId, you can get the entire amount of this token in all inputs 
    total_value_of_input_tokens: BTreeMap<TokenId, TransactionOutputFor<T>>,
    // Using TokenId, you can get the entire amount of this token in all outputs 
    total_value_of_output_tokens: BTreeMap<TokenId, TransactionOutputFor<T>>,
    // A set of transaction verification functions, this approach will allow you to remove unnecessary cycles, which will speed up the function 
    set_of_checks: Vec<&'a mut FnMut(...)>,
    // ...
    // I may add a priority field to the set of checks. I'm still thinking here. 
}
```

This struct we will use this way in the pallet utxo:

```rust
    pub fn validate_transaction<T: Config>(
        tx: &TransactionFor<T>,
    ) -> Result<ValidTransaction, &'static str> {
        TransactionVerifier::<'_, T>::new(tx)
            .checking_inputs()
            .checking_outputs()
            .checking_utxos_exists()
            .checking_signatures()
            .checking_tokens_transferring()
            .checking_tokens_issued()
            .checking_nft_mint()
            .checking_assets_burn()
            .calculating_reward()
            .collect_result()?
    }

```

When creating a new instance of this structure, we must initialize the fields. 

Each subsequent check adds a new instance of the function to `set_of_checks`, which will be called in` collect_result`. 

At the moment we can split the verification function for these parts: 

* `checking_inputs`
  * Checks that inputs exist in a transaction 
  * Checking that the number of inputs is not more than the maximum allowed number, now in the code I see that it is `u32::MAX` 
  * Ensure each input is used only a single time
  
* `checking_outputs`
  * Checks that outputs exist in a transaction 
  * Checking that the number of outputs is not more than the maximum allowed number, now in the code I see that it is `u32::MAX` 
  * Ensure each output is unique
  * Output value must be nonzero
  * An output can't exist already in the UtxoStore

* `checking_utxos_exists` 
  * Resolve the transaction inputs by looking up UTXOs being spent by them.

* `checking_signatures`
  * if all spent UTXOs are available, check the math and signatures

* `checking_tokens_transferring`
  * We have to check that the total sum of input tokens is less or equal to output tokens. (Or just equal?) 
  * All inputs with such data code must be correctly mapped to outputs 
  * If NFT is sent we must not burn or lose data 

* `checking_tokens_issued`
  * We must check the correctness of the issued tokens
  * We have to check the length of `metadata_uri` and` ticker` 
  * We must check the correctness of `value` and `decimal` 

* `checking_nft_mint`
  * We have to check the uniqueness of digital data, only one NFT can refer to one object 
  * We have to check the length of `metadata_uri`

* `checking_assets_burn`
  * Is there burn more than possible?
  * Is there tocken_id exist for the burn?

* `calculating_reward`
  * Just collecting MLT for a transaction reward.

* `collect_result`
  * Call all of these functions in one loop.

## Questions
* Do we need other checks? 
* What is we need for checking Bitcoin Script? 
* What is we need for checking contracts?
* If we can check an output address here, and add a possibility to find in the UtxoStore by any address format, then we can remove `fn pick_utxo` and `fn send_to_address`. Isn't that?

I'm glad to see any suggestions or critics.