Flux Capacitor
==================================

Indexer for the NEAR protocol. Can be used for any contract living on NEAR.

We require a specific tx log encoded in json in order for Flux Capacitor to pickup your log and insert it into the database:

```json
{
    "type": "TABLE_NAME"
    "params": {
        "key": "value"
    }
}
```

Uses the [NEAR Indexer Framework](https://github.com/nearprotocol/nearcore/tree/master/chain/indexer).

Refer to the NEAR Indexer Framework README to learn how to run this example.
