from terra_utils import Account, Asset
import asyncio


async def test():
    account = Account()
    code_ids = await account.store_contracts()

    nft_distribution = await account.contract.create(
        code_ids["galaxy_nft_distribution"],
        owner=account.acc_address,
        cost=Asset.asset("uluna", amount="1000000", native=True),
        nft_contract=None,
        limit_per_address=5,
        nft_limit=10,
        response_seconds=1,
    )

    nft_contract = await account.contract.create(
        code_ids["cw721_base"],
        name="GalacticPunks",
        symbol="GLP",
        minter=nft_distribution,
    )

    await nft_distribution.set_nft_contract(contract_addr=nft_contract)

    # make 3 reservations
    await account.chain(
        *[nft_distribution.reserve_nft(_send={"uluna": "1000000"}) for _ in range(3)]
    )

    # print my reservations
    print(
        await nft_distribution.query.reservations_by_address(
            address=account.acc_address
        )
    )

    # get ids of valid reservations
    valid_reservations = await nft_distribution.query.valid_reservations(
        start_at=0, limit=32
    )

    distribution_status = await nft_distribution.query.distribution_status()
    print(distribution_status)
    assert distribution_status["valid_reservations_count"] == 3

    # fill reservations
    for res_id in valid_reservations:
        await nft_distribution.mint_nft(
            reservation_id=res_id,
            token_id=str(res_id),
            name=f"NFT number {res_id}",
        )

    # get ids of valid reservations
    valid_reservations = await nft_distribution.query.valid_reservations(
        start_at=0, limit=32
    )
    # should have no valid reservations because all filled:
    assert not valid_reservations

    distribution_status = await nft_distribution.query.distribution_status()
    assert distribution_status["valid_reservations_count"] == 0

    # confirm NFTs have actually been minted
    for res_id in "012":
        owner_response = await nft_contract.query.owner_of(token_id=res_id)
        assert owner_response["owner"] == account.acc_address

    print(await nft_distribution.query.distribution_status())


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(test())
