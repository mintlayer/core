# Token creation

```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
  "jsonrpc":"2.0",
  "id":1,
  "method":"/v1/tokens/create",
  "params": ["My Test Token", "MTT", 1000]
}'

```

# Issue new tokens
# List of available tokens
# Burn tokens 
# List tokens 

```bash
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d   '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"/v1/tokens/list",
    "params": []
}'
```