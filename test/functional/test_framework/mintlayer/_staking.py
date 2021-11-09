import substrateinterface


class Staking(object):

    """ Query the node for the staking ledger """
    def get_staking_ledger(self):
        query = self.substrate.query_map(
            module='Staking',
            storage_function='Ledger'
        )

        return ((h, o.value) for (h, o) in query)
