# Rewards

## Block Rewards 
For each block they produce, a validator receives a reward. This section explains how to determine the block reward.

Since block production is set to occur every minute, approximately 

1 block/minute * 60 minutes/hour * 24 hours/day * 365 days/year = **525,600 blocks** are created *per year*.

We define a *block-year* as a sequence of 525,600 consecutive block indices. In this way, the block-year changes approximately every calendar year.

During the first block-year (i.e. for the first 525,600 blocks created), a fixed amount of 100 MLT tokens per block produced is awarded.
Every block-year thereafter, the reward per block drops by 25%, with a limit of 0.1 MLT tokens per block.

| Block-year | Reward per block (in MLT tokens) |
| ---------- | -------------------------------- |
| 1          | 100                              |
| 2          | 75                               |
| 3          | 50                               |
| 4          | 25                               |
| 5+         | 0.1                              |


## Transaction Fees
The transaction fees for UTXO spending and `withdraw_stake` also go to the block author.  
The `unlock_request_for_withdrawal` is free.
