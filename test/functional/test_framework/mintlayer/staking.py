import substrateinterface


class Staking(object):
    def __init__(self, substrate):
        self.substrate = substrate

    """ Query the node for the staking ledger """
    def get_staking_ledger(self):
        query = self.substrate.query_map(
            module='Staking',
            storage_function='Ledger'
        )

        return ((h, o.value) for (h, o) in query)

    """ accesses current era """
    def current_era(self):
        query = self.substrate.query(
            module='Staking',
            storage_function='CurrentEra'
        )
        # this query returns an object of type scalecodec.types.U32
        # so we convert it to an integer
        # TODO find a more elegant way to do conversion
        return int("{}".format(query))

    """ gets the staking ledger of the given key """
    def get_ledger(self, keypair):
        query = self.substrate.query_map(
            module='Staking',
            storage_function='Ledger'
        )

        matching = lambda e: e[0].value == keypair.ss58_address

        return filter(matching, ((h, o.value) for (h, o) in query))

    """ returns what era for the user to be able to withdraw funds """
    def withdrawal_era(self, keypair):
        ledger = list(self.get_ledger(keypair))
        if ledger:
            return ledger[0][1]['unlocking'][0]['era']
        else:
            print("no funds to withdraw")
