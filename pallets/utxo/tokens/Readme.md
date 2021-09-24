# Token creation

Call the extrinsic: 
```bash
* Creator - Alice 
* Pubkey - 0x2e1e60ac02d5a716b300e83b04bb4ddd48360ea119f5024f0ea7b2b1c1578a52
* Input - we will take Fee over here
* Token name - any value
* Token ticker - any value
* Supply - any value
```

# Request the tokens list

Call the RPC:

```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"tokens_list",
    "params": []
}'
```