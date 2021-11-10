# Staking

Mintlayer borrows from Substrate's [staking system](https://github.com/paritytech/substrate/blob/master/frame/staking/README.md), minus the elections and nominators. 
This means being a validator is as simple as locking your utxos: the higher your stake, the higher your chances of being chosen.

A few terms to recall before we proceed:
- controller account 
- stash account
- session key

Substrate has a documentaton about these [account abstractions](https://docs.substrate.io/v3/concepts/account-abstractions/) to help us understand the reasons behind this design.

#### Sessions, Era and Bonding Duration
* [*Session*](https://paritytech.github.io/substrate/latest/pallet_session/#terminology) - Substrate defines this as *a period of time (measured in blocks) that has a constant set of validators. Validators can only join or exit the validator set at a session change. A session is measured in block numbers*.
In Mintlayer, a session is currently set to **5 blocks**.
* *Era* - taken from Substrate's staking pallet *as a (whole) number of sessions, which is the period that the validator set is recalculated*. 
In Mintlayer, an era has been set to **2 sessions**.
* *Bonding Duration* - Once funds are unlocked, a duration (measured in eras) must pass until the funds can actually be withdrawn.
In Mintlayer, bonding duration is set to **2 eras**

### Locking UTXOs for Staking
In substrate's staking system, locking your funds invovlves several steps:
Firstly, bond your controller account to your stash account.
Secondly, [generate your session keys](https://docs.substrate.io/v3/concepts/session-keys/) *and* **set them up in your node**. See [key_management.md](key_management.md) for more details.
Lastly, [apply for the role of validator](https://docs.rs/pallet-staking/3.0.0/pallet_staking/#validating).
You will then be declared a candidate to be a validator for the next era.

These steps are the same for Mintlayer, but they are compounded into one *spend*:
1. Generate the session key, just as in Substrate.
2. In your signed transaction, use the destination **`LockForStaking`** and insert the `<controller_account>`, `<stash_account>`, `<session_key>`. 
3. Execute the *spend* call.

**Note**: The *minimum amount* to stake is **40,000 MLT**.
#### Locking Extra UTXOs for Staking
To lock additional funds, follow the same step as above, replacing **`LockForStaking`** with ***`LockExtraForStaking`***.
You do not need to supply your session key again.

#### Unlocking UTXOs for Withdrawal
Wanting to chill from validating and withdraw your locked utxos?
In Substrate, this involves two steps: *chill* and *unbond*.
Mintlayer makes it possible in one transaction call: **`unlock_request_for_withdrawal`** using the stash account.
Chilling is effective at the beginning of the next era. 
However, keep in mind the *bonding duration* (see above): for instance, if the unlocking is successful at 5th era,
then withdrawal becomes possible at the 7th era.

#### Withdraw UTXOs
Like Unlocking, withdrawal is done in a single call, **`withdraw_stake`**, using the stash account.
This is possible only after *bonding duration* has passed.
