🔎 Flux Capacitor
==================================

Indexer for the NEAR protocol. Can be used for any contract living on NEAR.

We require a specific tx log encoded in json in order for Flux Capacitor to pickup your log and insert it into the database:

```json
{
    "type": "TABLE_NAME",
    "cap_id": "ID_HERE",
    "action": "update or write",
    "params": {
        "key": "value"
    }
}
```

In order to run copy the `.env.example` to `.env` and run `docker-compose up`

Once started you can call using curl or via browser the following URL to tell Flux Capacitor to watch for logs for a specific contract:

http://localhost:3000/config/add_account?token=YOUR_API_TOKEN&account_id=CONTRACT_ID

Uses the [NEAR Indexer Framework](https://github.com/nearprotocol/nearcore/tree/master/chain/indexer).

Refer to the NEAR Indexer Framework README to learn how to run this example.
