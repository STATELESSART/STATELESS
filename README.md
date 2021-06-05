# STATELESS
Contracts for Stateless Art

Current contract implements a function, DAOpay, which currently takes as input (manual for now, unfortunately) a list of DAO members addresses.

The contract then calculates an equal split payment of amount transferred to the contract and sends the member share to each DAO member.

TODO: Automate!  We should be able to dynamically get a list of DAO members & we should be able to trigger the execution of DAOpay every time the contract receives a transfer from the DAO address.

