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

    def mlt_utxos(self):
        return ((h, u) for (h, u) in self.utxos() if u.data is None)

    def mlt_balance(self):
        return sum(u.value for (_, u) in self.mlt_utxos()) / MLT_UNIT

def balance(args):
    print('Total:', Account(args).mlt_balance(), 'MLT')

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

def pay(args):
    acct = Account(args)
    utxo_value = Decimal()
    utxos = []
    amount = int(args.amount * MLT_UNIT)

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
    acct.client.submit(acct.keypair, tx)

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
    pay_cmd.add_argument('key', type=str, metavar='SENDER_KEY',
            help='Sender private key')
    pay_cmd.add_argument('to', type=str, metavar='RECEPIENT_PUBKEY',
            help='Recepient public key')
    pay_cmd.add_argument('amount', type=Decimal, metavar='AMOUNT',
            help='Amount of MLT to send')

    return ap.parse_args()

def main():
    try:
        cmd = parse_command_line()
        cmd.func(cmd)
    except Exception as e:
        print(e)
        sys.exit(1)

if __name__ == '__main__':
    main()
