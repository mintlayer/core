#!/usr/bin/env python3
import os
import sys
sys.path.insert(0,
    os.path.join(os.path.dirname(__file__),
        '..', 'test', 'functional', 'test_framework', 'mintlayer'))

import argparse
import logging
from decimal import Decimal
from substrateinterface import Keypair

MLT_UNIT = Decimal('1e11')

try:
    import utxo as mint
except ImportError as e:
    print(e)
    print("Hint: try installing scalecodec and substrateinterface:")
    print("  python3 -m pip install 'substrate-interface == 0.13.12'")
    print("  python3 -m pip install 'scalecodec == 0.11.18'")
    sys.exit(1)

class Account:
    """ Represents a Substrate connection associated with an account """

    def __init__(self, args):
        self.keypair = Keypair.create_from_uri(args.key)
        self.client = mint.Client('ws://' + args.host, args.port)

    def utxos(self):
        return self.client.utxos_for(self.keypair)

    def locked_mlt_utxos(self):
        return self.client.locked_utxos_for(self.keypair)

    def mlt_utxos(self):
        return ((h, u) for (h, u) in self.utxos() if u.data is None)

    def locked_mlt_balance(self):
        res = sum(u.value for (_, u) in self.locked_mlt_utxos())
        if res == 0:
            return 0
        else:
            return sum(u.value for (_, u) in self.locked_mlt_utxos()) / MLT_UNIT

    def mlt_balance(self):
        return sum(u.value for (_, u) in self.mlt_utxos()) / MLT_UNIT

    def withdrawal_era(self):
       return self.client.withdrawal_era(self.keypair)

    def current_era(self):
       return self.client.current_era()

    def submit(self, tx):
        call = self.client.substrate.compose_call(
            call_module = 'Utxo',
            call_function = 'spend',
            call_params = { 'tx': tx.json() },
        )
        extrinsic = self.client.substrate.create_unsigned_extrinsic(call=call)

        def result_handler(message, update_nr, sub_id):
            if 'params' in message and type(message['params']['result']) is dict:
                res = message['params']['result']
                if 'inBlock' in res:
                    return {
                        'block_hash': res['inBlock']
                    }

        result = self.client.substrate.rpc_request(
            "author_submitAndWatchExtrinsic",
            [str(extrinsic.data)],
            result_handler=result_handler
        )

        if 'block_hash' in result:
            print('Transaction included in block', result['block_hash'])

        return result


def balance(args):
    print('Total Free:', Account(args).mlt_balance(), 'MLT')
    print('Total Locked:', Account(args).locked_mlt_balance(), 'MLT')

def print_key_info(keypair):
    print('Seed hex   :', keypair.seed_hex or 'UNKNOWN')
    print('Private key:', keypair.private_key)
    print('Public key :', keypair.public_key)
    print('Address    :', keypair.ss58_address)

def keyinfo(args):
    print_key_info(Keypair.create_from_uri(args.key))

def keygen(args):
    mnemonic = Keypair.generate_mnemonic(words=args.num_words)
    keypair = Keypair.create_from_uri(mnemonic)
    print('Mnemonic   :', mnemonic)
    print_key_info(keypair)

def unlock(args):
    acct = Account(args)
    acct.client.unlock_request_for_withdrawal(acct.keypair)

def withdrawal_era(args):
    account = Account(args)
    print("withdrawal era: ",account.withdrawal_era())
    print("current era: ",account.current_era())

def withdraw(args):
    acct = Account(args)
    acct.client.withdraw_stake(acct.keypair)


def lock(args):
    acct = Account(args)
    utxo_value = Decimal()
    utxos = []
    amount = int(args.amount * MLT_UNIT)

    if amount < 0:
        raise Exception('Sending a negative amount')
    elif amount <  40000:
        raise Exception('Minimum amount to stake is 40,000 MLT')

    for (h, u) in acct.mlt_utxos():
        utxos.append((h, u))
        utxo_value += u.value
        if utxo_value >= amount:
            break
    if utxo_value < amount:
        raise Exception('Not enough funds')

    fee = int(MLT_UNIT / 10) # TODO
    if fee >= 2 ** 64:
        raise Exception('Fee too large')

    change = utxo_value - amount - fee

    tx = mint.Transaction(
        acct.client,
        inputs=[ mint.Input(h) for (h, _) in utxos ],
        outputs=[
            mint.Output(
                value=amount,
                destination=mint.DestLockForStaking(acct.keypair.public_key, args.controller_key, args.session_key),
                data=None
            ),
            mint.Output(
                value=change,
                destination=mint.DestPubkey(acct.keypair.public_key),
                data=None
            ),
        ]
    ).sign(acct.keypair, [u for (_, u) in utxos])
    acct.submit(tx)


def lock_extra(args):
    acct = Account(args)
    utxo_value = Decimal()
    utxos = []
    amount = int(args.amount * MLT_UNIT)
    fee = int()

    if amount < 0:
        raise Exception('Sending a negative amount')

    for (h, u) in acct.mlt_utxos():
        utxos.append((h, u))
        utxo_value += u.value
        if utxo_value >= amount:
            break

    if utxo_value < amount:
        raise Exception('Not enough funds')

    fee = int(MLT_UNIT / 10) # TODO
    if fee >= 2 ** 64:
        raise Exception('Fee too large')

    change = utxo_value - amount - fee

    tx = mint.Transaction(
        acct.client,
        inputs=[ mint.Input(h) for (h, _) in utxos ],
        outputs=[
            mint.Output(
                value=amount,
                destination=mint.DestLockExtraForStaking(acct.keypair.public_key, args.controller_key),
                data=None
            ),
            mint.Output(
                value=change,
                destination=mint.DestPubkey(acct.keypair.public_key),
                data=None
            ),
        ]
    ).sign(acct.keypair, [u for (_, u) in utxos])
    acct.submit(tx)


def pay(args):
    acct = Account(args)
    utxo_value = Decimal()
    utxos = []
    amount = int(args.amount * MLT_UNIT)
    fee = int(args.fee * MLT_UNIT)
    total = amount + fee

    if fee >= 2 ** 64:
        raise Exception('Fee too high')

    if amount < 0:
        raise Exception('Sending a negative amount')

    for (h, u) in acct.mlt_utxos():
        utxos.append((h, u))
        utxo_value += u.value
        if utxo_value >= total:
            break

    change = utxo_value - amount - fee

    if change < 0:
        raise Exception('Not enough funds')

    tx = mint.Transaction(
        acct.client,
        inputs=[ mint.Input(h) for (h, _) in utxos ],
        outputs=[
            mint.Output(
                value=amount,
                destination=mint.DestPubkey(args.to),
                data=None
            ),
            mint.Output(
                value=change,
                destination=mint.DestPubkey(acct.keypair.public_key),
                data=None
            ),
        ]
    ).sign(acct.keypair, [u for (_, u) in utxos])
    acct.submit(tx)

def parse_command_line():
    ap = argparse.ArgumentParser(description='Mintlayer command line interface')
    ap.set_defaults(func=lambda x: ap.print_help())
    ap.add_argument('--host', '-H', type=str, default='127.0.0.1', metavar='NAME',
            help='Name or IP address of the node to connect to')
    ap.add_argument('--port', '-P', type=int, default=9944, metavar='PORT',
            help='Port of the node to connect to')

    sub = ap.add_subparsers(title='subcommands', metavar='')

    bal_cmd = sub.add_parser('balance', aliases=['b', 'bal'], help='Query balance')
    bal_cmd.set_defaults(func=balance)
    bal_cmd.add_argument('key', type=str, metavar='PUBKEY',
            help='Public key to query funds for')


    # TODO
    #bal.add_argument('--token', '-t', type=str, default=None, metavar='TOK',
    #        help='Filter by token type')

    key_cmd = sub.add_parser('keygen', help='Generate a new key')
    key_cmd.set_defaults(func=keygen)
    key_cmd.add_argument('--num-words', '-n', type=int, default=12,
            help='Number of seed words')

    keyinfo_cmd = sub.add_parser('keyinfo', help='Display info about a private key')
    keyinfo_cmd.set_defaults(func=keyinfo)
    keyinfo_cmd.add_argument('key', type=str, metavar='KEY',
            help='Private key')

    pay_cmd = sub.add_parser('pay', help='Submit a payment')
    pay_cmd.set_defaults(func=pay)
    pay_cmd.add_argument('--fee', type=Decimal, default=Decimal('0.1'), metavar='AMOUNT',
            help='Specify the amount of MLT paid in fees')
    pay_cmd.add_argument('key', type=str, metavar='SENDER_KEY',
            help='Sender private key')
    pay_cmd.add_argument('to', type=str, metavar='RECEPIENT_PUBKEY',
            help='Recepient public key')
    pay_cmd.add_argument('amount', type=Decimal, metavar='AMOUNT',
            help='Amount of MLT to send')

    lock_cmd = sub.add_parser('lock', help='Lock your utxos for first time staking.')
    lock_cmd.set_defaults(func=lock)
    lock_cmd.add_argument('key', type=str, metavar='STASH_KEY',
            help='STASH ACCOUNT private key')
    lock_cmd.add_argument('controller_key', type=str, metavar='CONTROLLER_PUB_KEY',
            help='CONTROLLER ACCOUNT public key')
    lock_cmd.add_argument('session_key', type=str, metavar='SESSION_KEY',
            help='session key after performing rpc call: `author_rotateKeys`')
    lock_cmd.add_argument('amount', type=Decimal, metavar='AMOUNT',
            help='Amount of MLT to send')

    lock_ex_cmd = sub.add_parser('lock_extra', help='Lock extra of your utxos for staking')
    lock_ex_cmd.set_defaults(func=lock_extra)
    lock_ex_cmd.add_argument('key', type=str, metavar='STASH_KEY',
            help='STASH ACCOUNT private key')
    lock_ex_cmd.add_argument('controller_key', type=str, metavar='CONTROLLER_PUB_KEY',
            help='CONTROLLER ACCOUNT public key')
    lock_ex_cmd.add_argument('amount', type=Decimal, metavar='AMOUNT',
            help='Amount of MLT to send')

    unlock_cmd = sub.add_parser('unlock', help='as a stash account, you request to unlock your staked utxos.')
    unlock_cmd.set_defaults(func=unlock)
    unlock_cmd.add_argument('key', type=str, metavar='STASH_KEY',
                help='Stash account private key')

    withdraw_cmd = sub.add_parser('withdraw', help='as a stash account, you want to withdraw your staked utxos.')
    withdraw_cmd.set_defaults(func=withdraw)
    withdraw_cmd.add_argument('key', type=str, metavar='STASH_KEY',
                help='Stash account private key')

    withdraw_era_cmd = sub.add_parser('withdrawal_era', help='given a controller account, returns the current era and the era you are able to withdraw')
    withdraw_era_cmd.set_defaults(func=withdrawal_era)
    withdraw_era_cmd.add_argument('key', type=str, metavar='CONTROLLER_KEY',
            help='Controller account private key')

    return ap.parse_args()

def main():
    try:
        cmd = parse_command_line()
        if hasattr(cmd, 'key') and os.path.isfile(cmd.key):
            with open(cmd.key) as f:
                cmd.key = f.readline().strip()
        cmd.func(cmd)
    except Exception as e:
        print(e)
        sys.exit(1)

if __name__ == '__main__':
    main()
